use std::convert::Infallible;

use log::info;

use hyper::header;
use hyper::{Body, Request, Response, StatusCode};

use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};

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
            if let Event::End(Tag::Heading(HeadingLevel::H1, _, _)) = events[2] {
                if let Event::Text(str) = &events[1] {
                    let ret = str.to_string();
                    events.drain(0..3);
                    return ret;
                }
            }
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
    file_name: &str,
    bytes: &[u8],
) -> Result<Response<Body>, MyError> {
    let markdown_input = std::str::from_utf8(bytes)?;
    let settings = wiki.settings();
    let markdown_page = MarkdownPage::new(settings, file_name, markdown_input);

    let title = markdown_page.title().to_owned();
    let rendered_markdown = markdown_page.render_html();

    let html_output = render_page(&title, &rendered_markdown)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=UTF-8")
        .body(Body::from(html_output))?)
}

struct RequestPathParts<'a> {
    pub path_elements: Vec<&'a str>,
    pub file_stem: &'a str,
    pub file_extension: &'a str,
}

impl<'a> RequestPathParts<'a> {
    pub fn parse(request_path: &'a str) -> Result<Self, MyError> {
        assert!(request_path.starts_with('/'));

        let path_elements: Vec<&str> = request_path[1..].split('/').collect();
        let (file_name, path_elements) = path_elements.split_last().unwrap();

        // TODO: support file names without file extensions
        let (file_stem, file_extension) = file_name.rsplit_once('.').unwrap();
        let path_elements: Vec<&'a str> = path_elements.into();
        Ok(RequestPathParts {
            path_elements,
            file_stem,
            file_extension,
        })
    }
}

async fn process_file_request(
    wiki: &Wiki,
    request_path: &str,
    byte: &[u8],
) -> Result<Response<Body>, MyError> {
    let path_info = RequestPathParts::parse(request_path)?;
    info!(
        "path_info: file_stem: {} file_ext: {}",
        path_info.file_stem, path_info.file_extension
    );

    match path_info.file_extension {
        "md" => markdown_response(
            wiki,
            // TODO: consider when these could fail and handle if needed.
            path_info.file_stem,
            byte,
        ),
        _ => Err(MyError::UnknownFilePath),
    }
}

async fn process_request_worker(
    wiki: &Wiki,
    req: &Request<Body>,
) -> Result<Response<Body>, MyError> {
    let file_path = req.uri().path();
    info!("start request: {}", file_path);

    let settings = wiki.settings();

    if file_path.ends_with('/') {
        info!(
            "Path '{:?}' appears to be a directory, appending {}.md",
            file_path,
            settings.index_page()
        );
        let mut file_path = file_path.to_owned();
        file_path += settings.index_page();
        file_path += ".md";
        return Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, file_path)
            .body(Body::empty())?);
    }

    match wiki.read_file(file_path) {
        Ok(bytes) => Ok(process_file_request(wiki, file_path, &bytes).await?),
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(format!("Path not found: {:?}", file_path)))?);
        }
    }
}

pub async fn process_request(wiki: Wiki, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_request_worker(&wiki, &req).await {
        Ok(res) => Ok(res),
        Err(err) => Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(header::CONTENT_TYPE, "text/plain; charset=UTF-8")
            .body(Body::from(format!("Failure processing request: {:?}", err)))
            .unwrap()),
    }
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

    fn assert_request_path_parse(
        input: &str,
        expected_file_stem: &str,
        expected_file_extension: &str,
        expected_path_elements: &[&str],
    ) {
        let parsed =
            RequestPathParts::parse(input).expect(&format!("Failed to parse request: {}", input));
        assert_eq!(
            expected_file_stem, parsed.file_stem,
            "Unexpected file_stem while parsing request: {}",
            input
        );
        assert_eq!(
            expected_file_extension, parsed.file_extension,
            "Unexpected file_extension while parsing request: {}",
            input
        );
        assert_eq!(
            expected_path_elements, parsed.path_elements,
            "Unexpected path_elements while parsing request: {}",
            input
        );
    }

    #[test]
    fn test_request_path_parse() {
        assert_request_path_parse("/README.md", "README", "md", &[]);
        assert_request_path_parse("/test/file.txt", "file", "txt", &["test"]);
        assert_request_path_parse(
            "/another/thing/to/test.markdown",
            "test",
            "markdown",
            &["another", "thing", "to"],
        );
    }
}
