use std::convert::Infallible;

use hyper::service::{make_service_fn, service_fn};
use hyper::Server;

mod error;
mod requests;
mod settings;
mod templates;

use requests::process_request;
use settings::parse_settings_from_args;

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    println!("got ctrl+c");
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    let settings = parse_settings_from_args()?;

    let make_svc = make_service_fn(|_conn| {
        // TODO: figure out how to give settings a static lifetime so cloning is not needed
        let settings = settings.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                process_request(settings.clone(), req)
            }))
        }
    });
    let addr = ([127, 0, 0, 1], 3000).into();
    let server = Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(shutdown_signal());

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
