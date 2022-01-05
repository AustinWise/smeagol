use std::convert::Infallible;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use log::info;

use hyper::header;
use hyper::{Body, Request, Response, StatusCode};

use pulldown_cmark::{html, Options};

use crate::error::MyError;
use crate::settings::Settings;
use crate::templates::render_page;

fn markdown_response(file_name: &str, file: &mut File) -> Result<Response<Body>, MyError> {
    let mut markdown_input = String::new();
    file.read_to_string(&mut markdown_input)?;

    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = pulldown_cmark::Parser::new_ext(&markdown_input, options);

    // Write to String buffer.
    let mut rendered_markdown = String::new();
    html::push_html(&mut rendered_markdown, parser);

    let html_output = render_page(file_name, &rendered_markdown)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=UTF-8")
        .body(Body::from(html_output))?)
}

async fn process_file_request(path_buf: &Path, file: &mut File) -> Result<Response<Body>, MyError> {
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
        // TODO: consider when these could fail and handle if needed.
        "md" => markdown_response(path_buf.file_name().unwrap().to_str().unwrap(), file),
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
        Ok(process_file_request(&path_buf, &mut f).await?)
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
            .body(Body::from(format!("Something went wrong: {:?}", err)))
            .unwrap()),
    }
}
