use std::convert::Infallible;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use log::info;

use hyper::header;
use hyper::{Body, Request, Response, StatusCode};

use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};

use crate::error::MyError;
use crate::settings::Settings;
use crate::templates::render_page;

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
    settings: &Settings,
    file_name: &str,
    file: &mut File,
) -> Result<Response<Body>, MyError> {
    let mut markdown_input = String::new();
    file.read_to_string(&mut markdown_input)?;
    let markdown_page = MarkdownPage::new(settings, file_name, &markdown_input);

    let title = markdown_page.title().to_owned();
    let rendered_markdown = markdown_page.render_html();

    let html_output = render_page(&title, &rendered_markdown)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=UTF-8")
        .body(Body::from(html_output))?)
}

async fn process_file_request(
    settings: &Settings,
    path_buf: &Path,
    file: &mut File,
) -> Result<Response<Body>, MyError> {
    let ext = match path_buf.extension() {
        None => return Err(MyError::BadPath),
        Some(ext) => ext,
    };

    let ext = match ext.to_str() {
        None => return Err(MyError::BadPath),
        Some(ext) => ext,
    };

    info!("f: {:?} ext: {}", path_buf, ext);

    match ext {
        "md" => markdown_response(
            settings,
            // TODO: consider when these could fail and handle if needed.
            path_buf.file_stem().unwrap().to_str().unwrap(),
            file,
        ),
        _ => Err(MyError::UnknownFilePath),
    }
}

async fn process_request_worker(
    settings: &Settings,
    req: &Request<Body>,
) -> Result<Response<Body>, MyError> {
    let file_path = req.uri().path();
    info!("start request: {}", file_path);
    // TODO: Figure out if there is any reason we would not get a slash.
    //       Convert to a 404 or 500 error if so.
    assert!(!file_path.is_empty() && &file_path[0..1] == "/");

    let mut path_buf = settings.git_repo().clone();
    path_buf.push(&file_path[1..]);

    let path_buf = match path_buf.canonicalize() {
        Ok(b) => b,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(format!("Path not found: {:?}", path_buf)))?);
        }
    };
    info!("canonicalized path: {:?}", path_buf);

    if !path_buf.starts_with(settings.git_repo()) {
        // TODO: Stronger resistance against path traversal attacks.
        // We are checking paths here, but ideally the operating system would
        // also have our back. Something like OpenBSD's `unveil(2)` could
        // prevent us from accessing files we did not intend to access.
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Path traversal attack."))?);
    }

    if path_buf.is_dir() {
        info!(
            "Path '{:?}' appears to be a directory, appending {}.md",
            path_buf,
            settings.index_page()
        );
        let mut file_path = file_path.to_owned();
        if !file_path.ends_with('/') {
            file_path += "/";
        }
        file_path += settings.index_page();
        file_path += ".md";
        Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, file_path)
            .body(Body::empty())?)
    } else {
        info!("Opening file: {:?}", path_buf);
        let mut f = File::open(&path_buf)?;
        Ok(process_file_request(settings, &path_buf, &mut f).await?)
    }
}

pub async fn process_request(
    settings: Settings,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    match process_request_worker(&settings, &req).await {
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
}
