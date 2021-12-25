use std::convert::Infallible;

use log::{info, trace, warn};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

use pulldown_cmark::{html, Options, Parser};


// TODO: make configurable
static ROOT: &str = "/home/austin/rustwiki/";

fn md() -> String {
    let markdown_input = "Hello world, this is a ~~complicated~~ *very simple* example.";

    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown_input, options);

    // Write to String buffer.
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

async fn process_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let file_path = req.uri().path();
    info!("path: {}", file_path);
    Ok(Response::new(Body::from(md())))
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