use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::info;

use hyper::header;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};

use pulldown_cmark::{html, Options, Parser};

// TODO: make configurable
static ROOT: &str = "/workspaces/rustwiki/test_site/";

#[derive(Debug)]
enum MyError {
    BadPath,
    UnknownFilePath,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::BadPath => write!(f, "bad path"),
            MyError::UnknownFilePath => write!(f, "unknown file type"),
        }
    }
}

impl Error for MyError {}

fn markdown_response(file: &mut File) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let mut markdown_input = String::new();
    file.read_to_string(&mut markdown_input)?;

    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&markdown_input, options);

    // Write to String buffer.
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(html_output))?)
}

async fn process_file_request(
    path_buf: &Path,
    file: &mut File,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let ext = match path_buf.extension() {
        None => return Err(Box::new(MyError::BadPath)),
        Some(ext) => ext,
    };

    let ext = match ext.to_str() {
        None => return Err(Box::new(MyError::BadPath)),
        Some(ext) => ext,
    };

    info!("f: {:?} ext: {}", path_buf, ext);

    match ext {
        "md" => markdown_response(file),
        _ => Err(Box::new(MyError::UnknownFilePath)),
    }
}

async fn process_request_inner(
    req: &Request<Body>,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let file_path = req.uri().path();
    info!("start request: {}", file_path);
    // TODO: Figure out if there is any reason we would not get a slash.
    //       Convert to a 404 or 500 error if so.
    assert!(!file_path.is_empty() && &file_path[0..1] == "/");

    let mut path_buf = PathBuf::from(ROOT);
    path_buf.push(&file_path[1..]);

    let path_buf = path_buf.canonicalize()?;
    info!("canonicalized path: {:?}", path_buf);

    if !path_buf.starts_with(ROOT) {
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

async fn process_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match process_request_inner(&req).await {
        Ok(res) => Ok(res),
        Err(_) => Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Something went wrong"))
            .unwrap()),
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(|_conn| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        async { Ok::<_, Infallible>(service_fn(process_request)) }
    });

    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
