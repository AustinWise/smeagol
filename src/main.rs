use std::convert::Infallible;

use hyper::service::{make_service_fn, service_fn};
use hyper::Server;

mod args;
mod requests;
mod error;

use args::Args;
use requests::process_request;

async fn run_server(args: Args) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For every connection, we must make a `Service` to handle all
    // incoming HTTP requests on said connection.
    let make_svc = make_service_fn(|_conn| {
        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.
        let args = args.clone();
        async { Ok::<_, Infallible>(service_fn(move |req| process_request(args.clone(), req))) }
    });

    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(make_svc);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    let args = Args::parse();

    run_server(args).await?;

    Ok(())
}
