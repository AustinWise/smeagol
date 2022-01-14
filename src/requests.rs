use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::Formatter;
use rocket::http::uri::fmt::Path;
use rocket::http::uri::fmt::UriDisplay;
use rocket::http::uri::Segments;
use rocket::request::FromSegments;
use rocket::response;
use rocket::response::content;
use rocket::response::Responder;
use rocket::{Build, Rocket};

use crate::error::MyError;
use crate::settings::Settings;
use crate::templates::render_page;
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
    fn parse(path_elements: &'a [String]) -> Result<Self, MyError> {
        let (file_name, path_elements) = path_elements.split_last().unwrap();

        // TODO: support file names without file extensions
        let (file_stem, file_extension) = file_name.rsplit_once('.').unwrap();
        Ok(RequestPathParts {
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

// Most of the time we are returning Content, so it is ok that it is bigger
#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
enum WikiPageResponder {
    Content(response::content::Html<String>),
    Redirect(response::Redirect),
    NotFound(response::status::NotFound<String>),
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

    fn to_parts(&self) -> Result<RequestPathParts, MyError> {
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

#[get("/page/<path..>")]
fn page(path: WikiPagePath, w: Wiki) -> WikiPageResponder {
    match w.read_file(&path.segments) {
        Ok(bytes) => {
            let path_info = path.to_parts().unwrap();
            match path_info.file_extension {
                "md" => WikiPageResponder::Content(response::content::Html(
                    markdown_response(&w, &path_info, &bytes).unwrap(),
                )),
                _ => WikiPageResponder::NotFound(response::status::NotFound(format!(
                    "File extension on this file is not supported: {}",
                    path
                ))),
            }
        }
        Err(_) => {
            if w.directory_exists(&path.segments).unwrap() {
                let mut segments = path.segments;
                segments.push(format!("{}.md", w.settings().index_page()));
                let path = WikiPagePath::new(segments);
                WikiPageResponder::Redirect(response::Redirect::to(uri!(page(path))))
            } else {
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

pub fn build_rocket() -> Rocket<Build> {
    rocket::build().mount("/", routes![primer_css, page, index])
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
}
