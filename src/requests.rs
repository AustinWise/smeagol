use std::borrow::Cow;

use rocket::form::Form;
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::Formatter;
use rocket::http::uri::fmt::Path;
use rocket::http::uri::fmt::UriDisplay;
use rocket::http::uri::Origin;
use rocket::http::uri::Segments;
use rocket::http::ContentType;
use rocket::request::FromSegments;
use rocket::response;
use rocket::response::Responder;
use rocket::{Build, Rocket};

use crate::error::MyError;
use crate::repository;
use crate::repository::RepositoryCapability;
use crate::templates;
use crate::templates::render_search_results;
use crate::templates::{
    render_edit_page, render_overview, render_page, render_page_placeholder, Breadcrumb,
};
use crate::wiki::Wiki;

// Most of the time we are returning Page, so it is ok that it is bigger
#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
enum WikiPageResponder {
    Page((ContentType, String)),
    File(Vec<u8>),
    TypedFile((ContentType, Vec<u8>)),
    Redirect(response::Redirect),
    NotFound(response::status::NotFound<String>),
    PagePlaceholder(response::status::NotFound<(ContentType, String)>),
}

#[derive(Debug)]
struct WikiPagePath<'r> {
    segments: Vec<&'r str>,
}

impl<'r> WikiPagePath<'r> {
    fn new(segments: Vec<&'r str>) -> Self {
        WikiPagePath { segments }
    }

    fn from_slice(segments: &[&'r str]) -> Self {
        WikiPagePath {
            segments: segments.to_vec(),
        }
    }

    fn append_segment(&self, new_seg: &'r str) -> Self {
        let mut segments = self.segments.clone();
        segments.push(new_seg);
        WikiPagePath { segments }
    }

    fn directories(&self) -> &[&'r str] {
        match self.segments.split_last() {
            Some((_, dirs)) => dirs,
            None => &[],
        }
    }

    fn directory(&self) -> Option<Self> {
        let (_, dirs) = self.segments.split_last()?;
        Some(WikiPagePath {
            segments: dirs.to_owned(),
        })
    }

    fn file_name(&self) -> Option<&str> {
        let (file_name, _) = self.segments.split_last()?;
        Some(file_name)
    }

    fn file_stem_and_extension(&self) -> Option<(&str, &str)> {
        let (file_name, _) = self.segments.split_last()?;
        file_name.rsplit_once('.')
    }

    #[cfg(test)]
    fn file_stem(&self) -> Option<&str> {
        Some(self.file_stem_and_extension()?.0)
    }

    #[cfg(test)]
    fn file_extension(&self) -> Option<&str> {
        Some(self.file_stem_and_extension()?.1)
    }

    fn breadcrumbs_helper<F: Fn(&'r [&'r str]) -> Origin>(
        &'r self,
        mut dirs: &'r [&'r str],
        uri_func: F,
    ) -> Vec<Breadcrumb<'r>> {
        let mut ret = Vec::with_capacity(dirs.len());
        while let Some((name, next_dirs)) = dirs.split_last() {
            let url = uri_func(dirs).to_string();
            ret.push(Breadcrumb::new(name, url));
            dirs = next_dirs;
        }
        //TODO: put the elements in the list in the correct order
        ret.reverse();
        ret
    }

    fn page_breadcrumbs(&'r self) -> Vec<Breadcrumb<'r>> {
        self.breadcrumbs_helper(self.directories(), |dirs| {
            uri!(page(WikiPagePath::from_slice(dirs)))
        })
    }

    fn overview_breadcrumbs(&'r self) -> Vec<Breadcrumb<'r>> {
        self.breadcrumbs_helper(&self.segments, |dirs| {
            uri!(overview(WikiPagePath::from_slice(dirs)))
        })
    }
}

impl<'r> FromSegments<'r> for WikiPagePath<'r> {
    type Error = MyError;

    fn from_segments(segments: Segments<'r, Path>) -> Result<Self, Self::Error> {
        let segments: Vec<&'r str> = segments.collect();
        Ok(WikiPagePath { segments })
    }
}

impl<'r> std::fmt::Display for WikiPagePath<'r> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.segments.is_empty() {
            write!(f, "/")?;
        } else {
            for path in &self.segments {
                write!(f, "/{}", path)?;
            }
        }
        Ok(())
    }
}

impl<'r> UriDisplay<Path> for WikiPagePath<'r> {
    fn fmt(&self, f: &mut Formatter<Path>) -> Result<(), std::fmt::Error> {
        for part in &self.segments {
            f.write_value(part)?;
        }
        Ok(())
    }
}

impl_from_uri_param_identity!([Path] ('r) WikiPagePath<'r>);

// TODO: is the an easier way to convert an Error into a 500?
impl<'r, 'o: 'r> Responder<'r, 'o> for MyError {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let str = format!("server error: {}", self);
        rocket::Response::build()
            .header(ContentType::Plain)
            .status(rocket::http::Status::InternalServerError)
            .sized_body(str.len(), std::io::Cursor::new(str))
            .ok()
    }
}

//TODO: implement a "real" cross-site request forgery solution.
// Ideally this would include some sort of expiration for the tokens.
// Though this might be good enough, because I expect in typical use the wiki
// server is shut down periodically, essentially expiring the tokens.
lazy_static! {
    static ref CSRF_TOKEN: String = {
        let bytes = rand::random::<[u8; 32]>();
        bytes.map(|b| format!("{:02x}", b)).concat()
    };
}

#[derive(FromForm)]
struct PageEditForm<'r> {
    content: &'r str,
    message: &'r str,
    authenticity_token: &'r str,
}

fn edit_save_inner(
    path: WikiPagePath,
    content: Form<PageEditForm<'_>>,
    w: Wiki,
) -> Result<response::Redirect, MyError> {
    if content.authenticity_token != *CSRF_TOKEN {
        return Err(MyError::Csrf);
    }
    let message = if content.message.trim().is_empty() {
        let message = default_edit_message(&path);
        Cow::Owned(message)
    } else {
        Cow::Borrowed(content.message.trim())
    };
    w.write_file(&path.segments, &message, content.content)?;
    Ok(response::Redirect::to(uri!(page(path))))
}

#[post("/edit/<path..>", data = "<content>")]
fn edit_save(
    path: WikiPagePath,
    content: Form<PageEditForm<'_>>,
    w: Wiki,
) -> Result<response::Redirect, MyError> {
    edit_save_inner(path, content, w)
}

fn default_edit_message(path: &WikiPagePath) -> String {
    format!("Update {}", path.file_name().expect("Ill-formed path"))
}

fn edit_view_inner(path: WikiPagePath, w: Wiki) -> Result<(ContentType, String), MyError> {
    let content = w.read_file(&path.segments).unwrap_or_else(|_| vec![]);
    let content = std::str::from_utf8(&content)?;
    let post_url = uri!(edit_save(&path));
    let view_url = uri!(page(&path));
    let preview_url = uri!(preview(&path));
    let title = format!("Editing {}", path.file_name().expect("Ill-formed path"));
    let message_placeholder = if w
        .repo_capabilities()
        .contains(RepositoryCapability::SUPPORTS_EDIT_MESSAGE)
    {
        Some(default_edit_message(&path))
    } else {
        None
    };
    let html = render_edit_page(
        &title,
        &post_url.to_string(),
        &view_url.to_string(),
        &preview_url.to_string(),
        message_placeholder,
        content,
        path.page_breadcrumbs(),
        &CSRF_TOKEN,
    )?;
    Ok((ContentType::HTML, html))
}

#[get("/edit/<path..>")]
fn edit_view(path: WikiPagePath, w: Wiki) -> Result<(ContentType, String), MyError> {
    edit_view_inner(path, w)
}

fn page_response(
    page: crate::page::Page,
    path: &WikiPagePath,
) -> Result<(ContentType, String), MyError> {
    let edit_url = uri!(edit_view(path)).to_string();
    let overview_url = uri!(overview(path.directory().unwrap())).to_string();
    let html = render_page(
        &page.title,
        &edit_url,
        &overview_url,
        &page.body,
        path.page_breadcrumbs(),
    )?;
    Ok((ContentType::HTML, html))
}

fn page_inner(path: WikiPagePath, w: Wiki) -> Result<WikiPageResponder, MyError> {
    match w.read_file(&path.segments) {
        Ok(bytes) => {
            let file_info = path.file_stem_and_extension();
            Ok(match file_info {
                Some((file_stem, file_ext)) => {
                    match crate::page::get_page(file_stem, file_ext, &bytes, w.settings())? {
                        Some(page_model) => {
                            WikiPageResponder::Page(page_response(page_model, &path)?)
                        }
                        None => match ContentType::from_extension(file_ext) {
                            Some(mine_type) => WikiPageResponder::TypedFile((mine_type, bytes)),
                            None => WikiPageResponder::File(bytes),
                        },
                    }
                }
                None => WikiPageResponder::File(bytes),
            })
        }
        Err(_) => {
            if w.directory_exists(&path.segments).unwrap() {
                let file_name = format!("{}.md", w.settings().index_page());
                let file_path = path.append_segment(&file_name);
                if w.file_exists(&file_path.segments)? {
                    Ok(WikiPageResponder::Redirect(response::Redirect::to(uri!(
                        page(file_path)
                    ))))
                } else {
                    Ok(WikiPageResponder::Redirect(response::Redirect::to(uri!(
                        overview(path)
                    ))))
                }
            } else {
                match path.file_stem_and_extension() {
                    Some((file_stem, "md")) => {
                        let create_url = uri!(edit_view(&path));
                        let overview_url = uri!(overview(path.directory().unwrap())).to_string();
                        Ok(WikiPageResponder::PagePlaceholder(
                            response::status::NotFound((
                                ContentType::HTML,
                                render_page_placeholder(
                                    file_stem,
                                    &path.to_string(),
                                    &create_url.to_string(),
                                    &overview_url,
                                    path.page_breadcrumbs(),
                                )
                                .unwrap(),
                            )),
                        ))
                    }
                    _ => Ok(WikiPageResponder::NotFound(response::status::NotFound(
                        format!("File not found: {}", path),
                    ))),
                }
            }
        }
    }
}

#[get("/page/<path..>")]
fn page(path: WikiPagePath, w: Wiki) -> Result<WikiPageResponder, MyError> {
    page_inner(path, w)
}

fn overview_inner(path: WikiPagePath, w: Wiki) -> Result<(ContentType, String), MyError> {
    let mut entries = w.enumerate_files(&path.segments)?;
    entries.sort();
    let entries = entries;

    let directories = entries
        .iter()
        .filter_map(|e| match e {
            repository::RepositoryItem::Directory(name) => {
                let url = uri!(overview(path.append_segment(name))).to_string();
                Some(templates::DirectoryEntry::new(name, url))
            }
            _ => None,
        })
        .collect();

    let files = entries
        .iter()
        .filter_map(|e| match e {
            repository::RepositoryItem::File(name) => {
                let url = uri!(page(path.append_segment(name))).to_string();
                Some(templates::DirectoryEntry::new(name, url))
            }
            _ => None,
        })
        .collect();

    let html = render_overview("Overview", path.overview_breadcrumbs(), directories, files)?;
    Ok((ContentType::HTML, html))
}

#[get("/overview/<path..>")]
fn overview(path: WikiPagePath, w: Wiki) -> Result<(ContentType, String), MyError> {
    overview_inner(path, w)
}

fn search_inner(q: &str, offset: Option<usize>, w: Wiki) -> Result<(ContentType, String), MyError> {
    const RESULTS_PER_PAGE: usize = 10;
    let results = w.search(q, RESULTS_PER_PAGE, offset)?;
    let prev_url = offset.and_then(|v| {
        if v >= RESULTS_PER_PAGE {
            Some(uri!(search(q, Some(v - RESULTS_PER_PAGE))).to_string())
        } else {
            None
        }
    });
    let next_url = Some(uri!(search(q, Some(offset.unwrap_or(0) + RESULTS_PER_PAGE))).to_string());
    let html = render_search_results(q, results, prev_url, next_url)?;
    Ok((ContentType::HTML, html))
}

#[get("/search?<q>&<offset>")]
fn search(q: &str, offset: Option<usize>, w: Wiki) -> Result<(ContentType, String), MyError> {
    search_inner(q, offset, w)
}

fn preview_inner(
    path: WikiPagePath,
    content: &str,
    w: Wiki,
) -> Result<(ContentType, String), MyError> {
    let (file_stem, file_extension) = path.file_stem_and_extension().unwrap();
    let page = crate::page::get_page(file_stem, file_extension, content.as_bytes(), w.settings())?;
    let page = page.unwrap();
    Ok((ContentType::HTML, page.body))
}

// TODO: add CSRF token
#[post("/preview/<path..>", data = "<content>")]
fn preview(path: WikiPagePath, content: &str, w: Wiki) -> Result<(ContentType, String), MyError> {
    preview_inner(path, content, w)
}

#[get("/")]
fn index(w: Wiki) -> response::Redirect {
    let file_name = format!("{}.md", w.settings().index_page());
    let path = WikiPagePath::new(vec![&file_name]);
    response::Redirect::to(uri!(page(path)))
}

pub fn mount_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount(
        "/",
        routes![page, search, edit_save, edit_view, preview, overview, index],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_request_path_parse(
        input: &[&'static str],
        expected_file_stem: &str,
        expected_file_extension: &str,
        expected_path_elements: &[&str],
    ) {
        let parsed = WikiPagePath::from_slice(input);
        assert_eq!(
            Some(expected_file_stem),
            parsed.file_stem(),
            "Unexpected file_stem while parsing request: {:?}",
            input
        );
        assert_eq!(
            Some(expected_file_extension),
            parsed.file_extension(),
            "Unexpected file_extension while parsing request: {:?}",
            input
        );
        assert_eq!(
            expected_path_elements,
            parsed.directories(),
            "Unexpected path_elements while parsing request: {:?}",
            input
        );
    }

    #[test]
    fn test_request_path_parse() {
        assert_request_path_parse(&["README.md"], "README", "md", &[]);
        assert_request_path_parse(&["test", "file.txt"], "file", "txt", &["test"]);
        assert_request_path_parse(
            &["another", "thing", "to", "test.markdown"],
            "test",
            "markdown",
            &["another", "thing", "to"],
        );
    }

    #[test]
    fn test_request_path_parse_unsupported() {
        let empty = WikiPagePath::new(vec![]);
        assert!(empty.directories().is_empty());
        assert!(empty.file_stem_and_extension().is_none());

        let extensionless_file = WikiPagePath::new(vec!["README"]);
        assert!(extensionless_file.directories().is_empty());
        assert!(extensionless_file.file_stem_and_extension().is_none());
    }

    #[test]
    fn test_wikipath_append() {
        let empty = WikiPagePath::new(vec![]);

        let folder = empty.append_segment("folder");
        assert_eq!(folder.segments, vec!["folder"]);

        let file = folder.append_segment("file");
        assert_eq!(file.segments, vec!["folder", "file"]);
    }
}
