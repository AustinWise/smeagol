#[macro_use]
extern crate rocket;

mod assets;
mod error;
mod page;
mod repository;
mod requests;
mod settings;
mod templates;
mod wiki;

use error::MyError;
use repository::create_file_system_repository;
use settings::parse_settings_from_args;
use wiki::Wiki;

use once_cell::sync::OnceCell;
use rocket::request::{FromRequest, Outcome, Request};

static WIKI: OnceCell<Wiki> = OnceCell::new();

fn create_wiki() -> Result<Wiki, MyError> {
    let settings = parse_settings_from_args()?;
    let repo = create_file_system_repository(settings.git_repo().clone())?;
    Ok(Wiki::new(settings, Box::new(repo)))
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
    let figment = rocket::Config::figment()
        .merge(("port", WIKI.get().unwrap().settings().port()))
        .merge(("address", WIKI.get().unwrap().settings().host()));
    let rocket = rocket::custom(figment);
    let rocket = requests::mount_routes(rocket);
    let rocket = assets::mount_routes(rocket);
    rocket.ignite().await?.launch().await
}
