use rocket::http::Header;
use rocket::Build;
use rocket::Rocket;

#[derive(Responder)]
#[response(status = 200)]
struct AssetResponse {
    content: &'static [u8],
    cache: Header<'static>,
    content_type: Header<'static>,
}

impl AssetResponse {
    fn new(content: &'static [u8], content_type: &'static str) -> Self {
        Self {
            content,
            cache: Header::new("Cache-Control", "public, max-age=604800"),
            content_type: Header::new("Content-Type", content_type),
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/assets.rs"));
