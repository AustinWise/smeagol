#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

mod assets;
mod error;
mod page;
mod repository;
mod requests;
mod settings;
mod templates;
mod wiki;

use clap::Parser;
use error::MyError;
use repository::create_repository;
use settings::parse_settings_from_args;
use wiki::Wiki;

fn create_wiki() -> Result<Wiki, MyError> {
    let args = settings::Args::parse();
    let git_repo = args
        .git_repo()
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("Loading wiki in {}", git_repo.display());

    let repo = create_repository(args.use_fs(), git_repo)?;
    let settings = parse_settings_from_args(args, &repo)?;
    Wiki::new(settings, repo)
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let wiki = match create_wiki() {
        Ok(wiki) => wiki,
        Err(err) => {
            eprintln!("Failed to create wiki: {}", err);
            std::process::exit(1);
        }
    };
    println!("Wiki loaded");

    let address = wiki.settings().host();
    let port = wiki.settings().port();

    let figment = rocket::Config::figment()
        .merge(("port", port))
        .merge(("address", address));
    let rocket = rocket::custom(figment);
    let rocket = rocket.manage(wiki);
    let rocket = requests::mount_routes(rocket);
    let rocket = assets::mount_routes(rocket);
    let rocket = rocket.ignite().await?;

    println!("Smeagol is listening on http://{address}:{port}/");

    let rocket = rocket.launch().await?;

    println!("Shutting down");

    drop(rocket);

    Ok(())
}
