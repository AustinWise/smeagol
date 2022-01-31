use std::sync::Arc;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::IndexWriter;
use tantivy::ReloadPolicy;
use tantivy::SnippetGenerator;

use crate::error::MyError;
use crate::page::get_raw_page;
use crate::page::is_page;
use crate::repository::RepoBox;
use crate::repository::Repository;
use crate::repository::RepositoryItem;
use crate::settings::Settings;

/// Wiki god object.
#[derive(Debug)]
struct WikiInner {
    settings: Settings,
    repository: RepoBox,
    index: Index,
}

// TODO: is there are away to share immutable global without the reference counting? A 'static lifetime somehow?
#[derive(Clone, Debug)]
pub struct Wiki(Arc<WikiInner>);

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
                        let bytes = repository.read_file(&path)?;
                        if let Ok(Some(page)) = get_raw_page(file_stem, file_ext, &bytes, settings)
                        {
                            let mut url = String::new();
                            for path in path {
                                url += "/";
                                url += path;
                            }

                            let mut doc = Document::default();
                            doc.add_text(search_fields.path, url);
                            doc.add_text(search_fields.title, page.title);
                            doc.add_text(search_fields.body, page.body);

                            index_writer.add_document(doc);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn create_index(settings: &Settings, repository: &RepoBox) -> Result<Index, MyError> {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("path", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT | STORED);
    let schema = schema_builder.build();
    //TODO: store on disk?
    let index = Index::create_in_ram(schema.clone());

    let mut index_writer = index.writer(50_000_000)?;

    let search_fields = SearchFields::from_schema(&schema);
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

impl Wiki {
    pub fn new(
        settings: Settings,
        repository: Box<dyn Repository + Send + Sync>,
    ) -> Result<Self, MyError> {
        let repo_box = RepoBox::new(repository);
        let index = create_index(&settings, &repo_box)?;
        let inner = WikiInner {
            settings,
            repository: repo_box,
            index,
        };
        Ok(Wiki(Arc::from(inner)))
    }

    pub fn settings(&self) -> &Settings {
        &self.0.settings
    }

    pub fn read_file(&self, file_path: &[&str]) -> Result<Vec<u8>, MyError> {
        self.0.repository.read_file(file_path)
    }

    pub fn write_file(&self, file_path: &[&str], content: &str) -> Result<(), MyError> {
        self.0.repository.write_file(file_path, content)
    }

    pub fn directory_exists(&self, path: &[&str]) -> Result<bool, MyError> {
        self.0.repository.directory_exists(path)
    }

    pub fn enumerate_files(&self, directory: &[&str]) -> Result<Vec<RepositoryItem>, MyError> {
        self.0.repository.enumerate_files(directory)
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, MyError> {
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

        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

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
                    .text()
                    .unwrap()
                    .to_owned();
                let path = doc
                    .get_first(fields.path)
                    .unwrap()
                    .text()
                    .unwrap()
                    .to_owned();
                let snippet_html = snippet.to_html();
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
