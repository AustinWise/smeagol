use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::service::{make_service_fn, service_fn};
use hyper::Server;

mod error;
mod repository;
mod requests;
mod settings;
mod templates;
mod wiki;

use repository::create_file_system_repository;
use requests::process_request;
use settings::parse_settings_from_args;

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    let settings = parse_settings_from_args()?;
    let repo = create_file_system_repository(settings.git_repo().clone())?;
    let wiki = wiki::Wiki::new(settings, Box::new(repo));

    let make_svc = make_service_fn(|_conn| {
        // TODO: figure out how to give settings a static lifetime so cloning is not needed
        let wiki = wiki.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                process_request(wiki.clone(), req)
            }))
        }
    });
    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();
    let server = Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(shutdown_signal());

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
