use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::Formatter;
use rocket::http::uri::fmt::Path;
use rocket::http::uri::fmt::UriDisplay;
use rocket::http::uri::Segments;
use rocket::http::ContentType;
use rocket::request::FromSegments;
use rocket::response;
use rocket::response::content;
use rocket::response::Responder;
use rocket::{Build, Rocket};

use crate::error::MyError;
use crate::settings::Settings;
use crate::templates::{render_edit_page, render_page, render_page_placeholder};
use crate::wiki::Wiki;

struct MarkdownPage<'a> {
    title: String,
    events: Vec<Event<'a>>,
}

fn try_get_h1_title(
    settings: &Settings,
    fallback_file_name: &str,
    events: &mut Vec<Event>,
) -> String {
    if settings.h1_title() && events.len() >= 2 {
        if let Event::Start(Tag::Heading(HeadingLevel::H1, _, _)) = events[0] {
            let end_ndx = events
                .iter()
                .position(|e| matches!(e, Event::End(Tag::Heading(HeadingLevel::H1, _, _))))
                .unwrap();
            let title = events[1..end_ndx]
                .iter()
                .map(|e| match e {
                    Event::Text(str) => str.to_string(),
                    not_str => panic!("Expected str, got: {:?}", not_str),
                })
                .collect();
            events.drain(0..=end_ndx);
            return title;
        }
    }
    fallback_file_name.to_owned()
}

impl<'a> MarkdownPage<'a> {
    fn new(settings: &'a Settings, file_name: &'a str, src: &'a str) -> MarkdownPage<'a> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        let mut events: Vec<Event<'a>> = Parser::new_ext(src, options).collect();
        let title = try_get_h1_title(settings, file_name, &mut events);

        MarkdownPage { title, events }
    }

    fn title(&'a self) -> &'a str {
        &self.title
    }

    fn render_html(self) -> String {
        let mut rendered_markdown = String::new();
        html::push_html(&mut rendered_markdown, self.events.into_iter());
        rendered_markdown
    }
}

fn markdown_response(
    wiki: &Wiki,
    edit_url: &str,
    path: &RequestPathParts,
    bytes: &[u8],
) -> Result<String, MyError> {
    let markdown_input = std::str::from_utf8(bytes)?;
    let settings = wiki.settings();
    let markdown_page = MarkdownPage::new(settings, path.file_stem, markdown_input);

    let title = markdown_page.title().to_owned();
    let rendered_markdown = markdown_page.render_html();

    Ok(render_page(
        &title,
        edit_url,
        &rendered_markdown,
        path.path_elements,
    )?)
}

#[derive(Debug)]
struct RequestPathParts<'a> {
    pub path_elements: &'a [String],
    pub file_stem: &'a str,
    pub file_extension: &'a str,
}

impl<'a> RequestPathParts<'a> {
    /// Does not support empty paths
    fn parse(path_elements: &'a [String]) -> Option<Self> {
        let (file_name, path_elements) = path_elements.split_last()?;

        // TODO: support file names without file extensions
        let (file_stem, file_extension) = file_name.rsplit_once('.')?;
        Some(RequestPathParts {
            path_elements,
            file_stem,
            file_extension,
        })
    }
}

#[get("/_smeagol/primer.css")]
fn primer_css() -> content::Css<&'static str> {
    content::Css(include_str!("primer.css"))
}

// Most of the time we are returning Page, so it is ok that it is bigger
#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
enum WikiPageResponder {
    Page(response::content::Html<String>),
    File(Vec<u8>),
    TypedFile(response::content::Custom<Vec<u8>>),
    Redirect(response::Redirect),
    NotFound(response::status::NotFound<String>),
    PagePlaceholder(response::status::NotFound<response::content::Html<String>>),
}

#[derive(Debug)]
struct WikiPagePath {
    //TODO: maybe don't copy all the strings...
    segments: Vec<String>,
}

impl WikiPagePath {
    fn new(segments: Vec<String>) -> Self {
        WikiPagePath { segments }
    }

    fn to_parts(&self) -> Option<RequestPathParts> {
        RequestPathParts::parse(&self.segments)
    }
}

impl<'r> FromSegments<'r> for WikiPagePath {
    type Error = MyError;

    fn from_segments(segments: Segments<'r, Path>) -> Result<Self, Self::Error> {
        let segments: Vec<String> = segments.map(|s| s.to_owned()).collect();
        Ok(WikiPagePath { segments })
    }
}

impl std::fmt::Display for WikiPagePath {
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

impl UriDisplay<Path> for WikiPagePath {
    fn fmt(&self, f: &mut Formatter<Path>) -> Result<(), std::fmt::Error> {
        for part in &self.segments {
            f.write_value(part)?;
        }
        Ok(())
    }
}

impl_from_uri_param_identity!([Path] WikiPagePath);

// TODO: is the an easier way to convert an Error into a 500?
impl<'r, 'o: 'r> Responder<'r, 'o> for MyError {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let str = format!("server error: {:?}", self);
        rocket::Response::build()
            .header(ContentType::Plain)
            .status(rocket::http::Status::InternalServerError)
            .sized_body(str.len(), std::io::Cursor::new(str))
            .ok()
    }
}

#[post("/edit/<path..>")]
fn edit_save(path: WikiPagePath, w: Wiki) -> String {
    format!("SAVE NYI: {}", path)
}

#[get("/edit/<path..>")]
fn edit_view(path: WikiPagePath, w: Wiki) -> Result<response::content::Html<String>, MyError> {
    let content = w.read_file(&path.segments)?;
    let content = std::str::from_utf8(&content)?;
    let post_url = uri!(edit_save(&path));
    let view_url = uri!(page(&path));
    let path_info = path.to_parts().expect("I'll formed path");
    let html = render_edit_page(
        path_info.file_stem,
        &post_url.to_string(),
        &view_url.to_string(),
        content,
        path_info.path_elements,
    )?;
    Ok(response::content::Html(html))
}

#[get("/page/<path..>")]
fn page(path: WikiPagePath, w: Wiki) -> WikiPageResponder {
    match w.read_file(&path.segments) {
        Ok(bytes) => {
            let mut file_extension = None;
            if let Some(path_info) = path.to_parts() {
                if path_info.file_extension == "md" {
                    let edit_url = uri!(edit_view(&path)).to_string();
                    return WikiPageResponder::Page(response::content::Html(
                        markdown_response(&w, &edit_url, &path_info, &bytes).unwrap(),
                    ));
                }
                file_extension = Some(path_info.file_extension);
            }
            match file_extension.and_then(ContentType::from_extension) {
                Some(ext) => WikiPageResponder::TypedFile(response::content::Custom(ext, bytes)),
                None => WikiPageResponder::File(bytes),
            }
        }
        Err(_) => {
            if w.directory_exists(&path.segments).unwrap() {
                let mut segments = path.segments;
                segments.push(format!("{}.md", w.settings().index_page()));
                let path = WikiPagePath::new(segments);
                WikiPageResponder::Redirect(response::Redirect::to(uri!(page(path))))
            } else {
                if let Some(path_info) = path.to_parts() {
                    if path_info.file_extension == "md" {
                        let create_url = uri!(edit_view(&path));
                        return WikiPageResponder::PagePlaceholder(response::status::NotFound(
                            response::content::Html(
                                render_page_placeholder(
                                    path_info.file_stem,
                                    &path.to_string(),
                                    &create_url.to_string(),
                                    path_info.path_elements,
                                )
                                .unwrap(),
                            ),
                        ));
                    }
                }
                WikiPageResponder::NotFound(response::status::NotFound(format!(
                    "File not found: {}",
                    path
                )))
            }
        }
    }
}

#[get("/")]
fn index(w: Wiki) -> response::Redirect {
    let file_name = format!("{}.md", w.settings().index_page());
    let path = WikiPagePath::new(vec![file_name]);
    response::Redirect::to(uri!(page(path)))
}

pub fn mount_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/", routes![primer_css, page, edit_save, edit_view, index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_title() {
        let settings = Settings::new("Home", false);
        let input = "# First H1\n# Second H1";
        let markdown_page = MarkdownPage::new(&settings, "file_name", &input);
        assert_eq!("file_name", markdown_page.title());
        let rendered = markdown_page.render_html();
        assert_eq!("<h1>First H1</h1>\n<h1>Second H1</h1>\n", rendered);
    }

    #[test]
    fn test_h1_title() {
        let settings = Settings::new("Home", true);
        let input = "# First H1\n# Second H1";
        let markdown_page = MarkdownPage::new(&settings, "file_name", &input);
        assert_eq!("First H1", markdown_page.title());
        let rendered = markdown_page.render_html();
        assert_eq!("<h1>Second H1</h1>\n", rendered);
    }

    #[test]
    fn test_h1_title_complicated() {
        let settings = Settings::new("Home", true);
        let input = "# Austin\'s Wiki\nwords words words";
        let markdown_page = MarkdownPage::new(&settings, "file_name", &input);
        assert_eq!("Austin\u{2019}s Wiki", markdown_page.title());
        let rendered = markdown_page.render_html();
        assert_eq!("<p>words words words</p>\n", rendered);
    }

    fn assert_request_path_parse(
        input: &[&str],
        expected_file_stem: &str,
        expected_file_extension: &str,
        expected_path_elements: &[&str],
    ) {
        let input: Vec<String> = input.iter().map(|s| s.to_string()).collect();
        let parsed = RequestPathParts::parse(&input)
            .expect(&format!("Failed to parse request: {:?}", input));
        assert_eq!(
            expected_file_stem, parsed.file_stem,
            "Unexpected file_stem while parsing request: {:?}",
            input
        );
        assert_eq!(
            expected_file_extension, parsed.file_extension,
            "Unexpected file_extension while parsing request: {:?}",
            input
        );
        assert_eq!(
            expected_path_elements, parsed.path_elements,
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
        assert!(RequestPathParts::parse(&[]).is_none());
        assert!(RequestPathParts::parse(&["README".to_owned()]).is_none());
    }
}
