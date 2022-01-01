use std::convert::Infallible;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use log::info;

use hyper::header;
use hyper::{Body, Request, Response, StatusCode};

use pulldown_cmark::{html, Options};

use crate::args::Args;
use crate::error::MyError;

fn markdown_response(file: &mut File) -> Result<Response<Body>, MyError> {
    let mut markdown_input = String::new();
    file.read_to_string(&mut markdown_input)?;

    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = pulldown_cmark::Parser::new_ext(&markdown_input, options);

    // Write to String buffer.
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html")
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
        "md" => markdown_response(file),
        _ => Err(MyError::UnknownFilePath),
    }
}

async fn process_request_worker(
    args: &Args,
    req: &Request<Body>,
) -> Result<Response<Body>, MyError> {
    let file_path = req.uri().path();
    info!("start request: {}", file_path);
    // TODO: Figure out if there is any reason we would not get a slash.
    //       Convert to a 404 or 500 error if so.
    assert!(!file_path.is_empty() && &file_path[0..1] == "/");

    let mut path_buf = args.git_repo();
    path_buf.push(&file_path[1..]);

    let path_buf = path_buf.canonicalize()?;
    info!("canonicalized path: {:?}", path_buf);

    if !path_buf.starts_with(args.git_repo()) {
        // TODO: Stronger resistance against path traversal attacks.
        // We are checking paths here, but ideally the operating system would
        // also have our back. Something like OpenBSD's `pledge(2)` could
        // prevent us from accessing files we did not intend to access.
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Path traversal attack."))?);
    }

    // TODO: handle directories. Maybe redirect to README.md or show automatically?
    match File::open(&path_buf) {
        Ok(mut f) => process_file_request(&path_buf, &mut f).await,
        Err(_) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("File not found."))?),
    }
}

pub async fn process_request(args: Args, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_request_worker(&args, &req).await {
        Ok(res) => Ok(res),
        Err(err) => Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(format!("Something went wrong: {:?}", err)))
            .unwrap()),
    }
}