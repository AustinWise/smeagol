use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use rocket::form::Form;
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::Formatter;
use rocket::http::uri::fmt::Path;
use rocket::http::uri::fmt::UriDisplay;
use rocket::http::uri::Segments;
use rocket::http::ContentType;
use rocket::request::FromSegments;
use rocket::response;
use rocket::response::Responder;
use rocket::{Build, Rocket};

use crate::error::MyError;
use crate::settings::Settings;
use crate::templates::{render_edit_page, render_page, render_page_placeholder, Breadcrumb};
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
    path: &WikiPagePath,
    bytes: &[u8],
) -> Result<String, MyError> {
    let markdown_input = std::str::from_utf8(bytes)?;
    let settings = wiki.settings();
    let markdown_page = MarkdownPage::new(settings, path.file_name().unwrap(), markdown_input);

    let title = markdown_page.title().to_owned();
    let rendered_markdown = markdown_page.render_html();

    Ok(render_page(
        &title,
        edit_url,
        &rendered_markdown,
        path.create_breadcrumbs(),
    )?)
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
struct WikiPagePath<'r> {
    segments: Vec<&'r str>,
}

impl<'r> WikiPagePath<'r> {
    fn new(segments: Vec<&'r str>) -> Self {
        WikiPagePath { segments }
    }

    fn from_slice(segments: &[&'r str]) -> Self {
        WikiPagePath {
            segments: segments.iter().copied().collect(),
        }
    }

    fn directories(&self) -> &[&'r str] {
        match self.segments.split_last() {
            Some((_, dirs)) => dirs,
            None => &[],
        }
    }

    fn file_name_and_extension(&self) -> Option<(&str, &str)> {
        let (file_name, _) = self.segments.split_last()?;
        file_name.rsplit_once('.')
    }

    fn file_name(&self) -> Option<&str> {
        Some(self.file_name_and_extension()?.0)
    }

    fn file_extension(&self) -> Option<&str> {
        Some(self.file_name_and_extension()?.1)
    }

    fn create_breadcrumbs(&self) -> Vec<Breadcrumb<'r>> {
        let mut dirs = self.directories();
        let mut ret = Vec::with_capacity(dirs.len());
        while let Some((name, next_dirs)) = dirs.split_last() {
            let url = uri!(page(WikiPagePath::from_slice(dirs))).to_string();
            ret.push(Breadcrumb::new(name, url));
            dirs = next_dirs;
        }
        //TODO: put the elements in the list in the correct order
        ret.reverse();
        ret
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
        let str = format!("server error: {:?}", self);
        rocket::Response::build()
            .header(ContentType::Plain)
            .status(rocket::http::Status::InternalServerError)
            .sized_body(str.len(), std::io::Cursor::new(str))
            .ok()
    }
}

#[derive(FromForm)]
struct PageEditForm<'r> {
    content: &'r str,
}

#[post("/edit/<path..>", data = "<content>")]
fn edit_save(
    path: WikiPagePath,
    content: Form<PageEditForm<'_>>,
    w: Wiki,
) -> Result<response::Redirect, MyError> {
    w.write_file(&path.segments, content.content)?;
    Ok(response::Redirect::to(uri!(page(path))))
}

#[get("/edit/<path..>")]
fn edit_view(path: WikiPagePath, w: Wiki) -> Result<response::content::Html<String>, MyError> {
    let content = w.read_file(&path.segments).unwrap_or_else(|_| vec![]);
    let content = std::str::from_utf8(&content)?;
    let post_url = uri!(edit_save(&path));
    let view_url = uri!(page(&path));
    let file_stem = path.file_name().expect("Ill-formed path");
    let html = render_edit_page(
        file_stem,
        &post_url.to_string(),
        &view_url.to_string(),
        content,
        path.create_breadcrumbs(),
    )?;
    Ok(response::content::Html(html))
}

#[get("/page/<path..>")]
fn page(path: WikiPagePath, w: Wiki) -> WikiPageResponder {
    match w.read_file(&path.segments) {
        Ok(bytes) => {
            let file_extension = path.file_extension();
            if file_extension == Some("md") {
                let edit_url = uri!(edit_view(&path)).to_string();
                return WikiPageResponder::Page(response::content::Html(
                    markdown_response(&w, &edit_url, &path, &bytes).unwrap(),
                ));
            }
            match file_extension.and_then(ContentType::from_extension) {
                Some(ext) => WikiPageResponder::TypedFile(response::content::Custom(ext, bytes)),
                None => WikiPageResponder::File(bytes),
            }
        }
        Err(_) => {
            if w.directory_exists(&path.segments).unwrap() {
                let mut segments = path.segments;
                let file_name = &format!("{}.md", w.settings().index_page());
                segments.push(file_name);
                let path = WikiPagePath::new(segments);
                WikiPageResponder::Redirect(response::Redirect::to(uri!(page(path))))
            } else {
                match path.file_name_and_extension() {
                    Some((file_stem, "md")) => {
                        let create_url = uri!(edit_view(&path));
                        WikiPageResponder::PagePlaceholder(response::status::NotFound(
                            response::content::Html(
                                render_page_placeholder(
                                    file_stem,
                                    &path.to_string(),
                                    &create_url.to_string(),
                                    path.create_breadcrumbs(),
                                )
                                .unwrap(),
                            ),
                        ))
                    }
                    _ => WikiPageResponder::NotFound(response::status::NotFound(format!(
                        "File not found: {}",
                        path
                    ))),
                }
            }
        }
    }
}

#[get("/")]
fn index(w: Wiki) -> response::Redirect {
    let file_name = format!("{}.md", w.settings().index_page());
    let path = WikiPagePath::new(vec![&file_name]);
    response::Redirect::to(uri!(page(path)))
}

pub fn mount_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket.mount("/", routes![page, edit_save, edit_view, index])
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
        input: &[&'static str],
        expected_file_stem: &str,
        expected_file_extension: &str,
        expected_path_elements: &[&str],
    ) {
        let parsed = WikiPagePath::from_slice(input);
        assert_eq!(
            Some(expected_file_stem),
            parsed.file_name(),
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
        assert!(empty.file_name_and_extension().is_none());

        let extensionless_file = WikiPagePath::new(vec!["README"]);
        assert!(extensionless_file.directories().is_empty());
        assert!(extensionless_file.file_name_and_extension().is_none());
    }
}
