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

use once_cell::sync::OnceCell;
use rocket::request::{FromRequest, Outcome, Request};

static WIKI: OnceCell<Wiki> = OnceCell::new();

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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Wiki {
    type Error = MyError;

    async fn from_request(_req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(WIKI.get().unwrap().clone())
    }
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
    WIKI.set(wiki).expect("Failed to set global wiki pointer.");

    println!("Wiki loaded");

    let address = WIKI.get().unwrap().settings().host();
    let port = WIKI.get().unwrap().settings().port();

    let figment = rocket::Config::figment()
        .merge(("port", port))
        .merge(("address", address));
    let rocket = rocket::custom(figment);
    let rocket = requests::mount_routes(rocket);
    let rocket = assets::mount_routes(rocket);
    let rocket = rocket.ignite().await?;

    println!("Smeagol is listening on http://{address}:{port}/");

    let rocket = rocket.launch().await?;

    println!("Shutting down");

    drop(rocket);

    Ok(())
}
