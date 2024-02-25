use std::str;
use std::sync::Arc;

use lazy_static::lazy_static;

use regex::bytes::{Captures, Regex};

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::IndexWriter;
use tantivy::ReloadPolicy;
use tantivy::Snippet;
use tantivy::SnippetGenerator;

use crate::error::MyError;
use crate::page::get_raw_page;
use crate::page::is_page;
use crate::repository::RepoBox;
use crate::repository::RepositoryCapability;
use crate::repository::RepositoryItem;
use crate::settings::Settings;

/// Wiki god object.
struct WikiInner {
    settings: Settings,
    repository: RepoBox,
    index: Index,
}

// TODO: is there are away to share immutable global without the reference counting? A 'static lifetime somehow?
#[derive(Clone)]
pub struct Wiki(Arc<WikiInner>);

impl std::fmt::Debug for Wiki {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wiki").finish()
    }
}

struct SearchFields {
    title: Field,
    path: Field,
    body: Field,
}

impl SearchFields {
    fn from_schema(schema: &Schema) -> Self {
        let title = schema.get_field("title").unwrap();
        let path = schema.get_field("path").unwrap();
        let body = schema.get_field("body").unwrap();

        SearchFields { title, path, body }
    }
}

pub struct SearchResult {
    pub score: f32,
    pub title: String,
    pub path: String,
    pub snippet_html: String,
}

fn index_directory(
    settings: &Settings,
    repository: &RepoBox,
    index_writer: &mut IndexWriter,
    search_fields: &SearchFields,
    dir: Vec<&str>,
) -> Result<(), MyError> {
    for item in repository.enumerate_files(&dir)? {
        match item {
            RepositoryItem::Directory(subdir) => {
                let mut subdir_path = dir.clone();
                subdir_path.push(&subdir);
                index_directory(
                    settings,
                    repository,
                    index_writer,
                    search_fields,
                    subdir_path,
                )?;
            }
            RepositoryItem::File(file_name) => {
                if let Some((file_stem, file_ext)) = file_name.rsplit_once('.') {
                    if is_page(file_ext) {
                        let mut path = dir.clone();
                        path.push(&file_name);
                        let bytes = match repository.read_file(&path) {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                println!(
                                    "Failed to open file '{}' for indexing: {:?}",
                                    path.join("/"),
                                    err
                                );
                                continue;
                            }
                        };
                        match get_raw_page(file_stem, file_ext, &bytes, settings) {
                            Ok(Some(page)) => {
                                index_file(&path, search_fields, page, index_writer);
                            }
                            Ok(None) => {
                                unreachable!(
                                    "We should have already checked to see if this was a page."
                                );
                            }
                            Err(MyError::BadUtf8 { source }) => {
                                println!(
                                    "Bad UTF-8 in file, not indexing. See byte position {} in {}",
                                    source.valid_up_to(),
                                    path.join("/")
                                );
                            }
                            Err(err) => {
                                println!(
                                    "Failed to parse file '{}' for indexing: {}",
                                    path.join("/"),
                                    err
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn index_file(
    path: &[&str],
    search_fields: &SearchFields,
    page: crate::page::Page,
    index_writer: &mut IndexWriter,
) {
    let mut url = String::new();
    for path in path {
        url += "/";
        url += path;
    }
    let mut doc = Document::default();
    doc.add_text(search_fields.path, &url);
    doc.add_text(search_fields.title, page.title);
    doc.add_text(search_fields.body, page.body);
    index_writer.delete_term(Term::from_field_text(search_fields.path, &url));
    index_writer.add_document(doc).unwrap();
}

const INDEXING_HEAP_SIZE: usize = 50_000_000;

fn create_index(settings: &Settings, repository: &RepoBox) -> Result<Index, MyError> {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("path", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT | STORED);
    let schema = schema_builder.build();
    //TODO: store on disk?
    let index = Index::create_in_ram(schema.clone());

    let mut index_writer = index.writer(INDEXING_HEAP_SIZE)?;
    let search_fields = SearchFields::from_schema(&schema);

    println!("Indexing files, this can take a while if there are a lot.");
    index_directory(
        settings,
        repository,
        &mut index_writer,
        &search_fields,
        vec![],
    )?;
    index_writer.commit()?;

    Ok(index)
}

// TODO: this does not belong at all in the Wiki, it belongs more in request handling
fn highlight(snippet: Snippet) -> String {
    let mut result = String::new();
    let mut start_from = 0;

    for fragment_range in snippet.highlighted() {
        result.push_str(&snippet.fragment()[start_from..fragment_range.start]);
        result.push_str(
            "<span class=\"color-bg-accent-emphasis color-fg-on-emphasis p-1 rounded mb-4\">",
        );
        result.push_str(&snippet.fragment()[fragment_range.clone()]);
        result.push_str("</span>");
        start_from = fragment_range.end;
    }

    result.push_str(&snippet.fragment()[start_from..]);
    result
}

impl Wiki {
    pub fn new(settings: Settings, repository: RepoBox) -> Result<Self, MyError> {
        let index = create_index(&settings, &repository)?;
        let inner = WikiInner {
            settings,
            repository,
            index,
        };
        Ok(Wiki(Arc::from(inner)))
    }

    pub fn settings(&self) -> &Settings {
        &self.0.settings
    }

    pub fn repo_capabilities(&self) -> RepositoryCapability {
        self.0.repository.capabilities()
    }

    pub fn read_file(&self, file_path: &[&str]) -> Result<Vec<u8>, MyError> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"\[\[(.+?)\]\]").unwrap();
        }
        let mut res = self.0.repository.read_file(file_path);
        if let Ok(mut bytes) = res {
            while RE.is_match(&bytes) {
                bytes = RE.replace(&bytes, |caps: &Captures| {
                    if let Ok(filename) = str::from_utf8(&caps[1]) {
                        self.0.repository.read_file(&[filename]).unwrap_or(b"**read error**".to_vec())
                    } else {
                        b"**conversion error**".to_vec()
                    }
                }).to_vec();
            }
            res = Ok(bytes)
        }
        res
    }

    pub fn write_file(
        &self,
        file_path: &[&str],
        message: &str,
        content: &str,
    ) -> Result<(), MyError> {
        self.0.repository.write_file(file_path, message, content)?;

        if let Some((file_stem, file_ext)) = file_path.last().unwrap().rsplit_once('.') {
            if is_page(file_ext) {
                let mut writer = self.0.index.writer(INDEXING_HEAP_SIZE)?;
                let search_fields = SearchFields::from_schema(&self.0.index.schema());
                let page = get_raw_page(file_stem, file_ext, content.as_bytes(), &self.0.settings)?
                    .unwrap();
                index_file(file_path, &search_fields, page, &mut writer);
                writer.commit()?;
            }
        }

        Ok(())
    }

    pub fn directory_exists(&self, path: &[&str]) -> Result<bool, MyError> {
        self.0.repository.directory_exists(path)
    }

    pub fn file_exists(&self, path: &[&str]) -> Result<bool, MyError> {
        self.0.repository.file_exists(path)
    }

    pub fn enumerate_files(&self, directory: &[&str]) -> Result<Vec<RepositoryItem>, MyError> {
        self.0.repository.enumerate_files(directory)
    }

    pub fn search(
        &self,
        query: &str,
        num_results: usize,
        offset: Option<usize>,
    ) -> Result<Vec<SearchResult>, MyError> {
        let reader = self
            .0
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let searcher = reader.searcher();
        let fields = SearchFields::from_schema(&self.0.index.schema());
        let query_parser =
            QueryParser::for_index(&self.0.index, vec![fields.path, fields.title, fields.body]);

        let query = query_parser.parse_query(query)?;

        let mut top_docs = TopDocs::with_limit(num_results);
        if let Some(offset) = offset {
            top_docs = top_docs.and_offset(offset);
        }
        let top_docs = searcher.search(&query, &top_docs)?;

        let snippet_generator = SnippetGenerator::create(&searcher, &*query, fields.body)?;

        Ok(top_docs
            .iter()
            .filter_map(|(score, doc_address)| {
                let doc = match searcher.doc(*doc_address) {
                    Ok(doc) => doc,
                    Err(_) => return None,
                };
                let snippet = snippet_generator.snippet_from_doc(&doc);

                let score = *score;
                let title = doc
                    .get_first(fields.title)
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .to_owned();
                let path = doc
                    .get_first(fields.path)
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .to_owned();
                let snippet_html = highlight(snippet);
                Some(SearchResult {
                    score,
                    title,
                    path,
                    snippet_html,
                })
            })
            .collect())
    }
}
