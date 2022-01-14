#[macro_use] extern crate rocket;

use std::sync::Once;

mod error;
mod repository;
mod requests;
mod settings;
mod templates;
mod wiki;

use error::MyError;
use repository::create_file_system_repository;
use settings::parse_settings_from_args;
use wiki::Wiki;
use requests::build_rocket;

use rocket::request::{Request, FromRequest, Outcome};

static mut WIKI: Option<Wiki> = None;
static INIT: Once = Once::new();

fn create_wiki() -> Result<Wiki, MyError> {
    let settings = parse_settings_from_args()?;
    let repo = create_file_system_repository(settings.git_repo().clone())?;
    Ok(Wiki::new(settings, Box::new(repo)))
}

fn get_wiki() -> Wiki {
    unsafe {
        INIT.call_once(|| {
            // TODO: do something more useful with the error message
            WIKI = Some(create_wiki().unwrap());
        });
        WIKI.as_ref().unwrap().clone()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Wiki {
    type Error = MyError;

    async fn from_request(_req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(get_wiki())
    }
}

#[launch]
fn rocket() -> _ {
    // observe the panic
    let _ = get_wiki();
    build_rocket()
}