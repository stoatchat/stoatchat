use revolt_database::util::reference::Reference;
use rocket::http::ContentType;
use rocket::response::{self, Responder};
use rocket::{Request, Response};

#[derive(ToSchema)]
#[schema(format = "binary", value_type = String)]
pub struct CachedFile(Vec<u8>);

pub static CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

impl<'r> Responder<'r, 'static> for CachedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        Response::build_from(self.0.respond_to(req)?)
            .raw_header("Cache-Control", CACHE_CONTROL)
            .ok()
    }
}

/// Fetch Default Avatar
///
/// This returns a default avatar based on the given id.
#[utoipa::path(
    tag = "User Information",
    security(("Session-Token" = []), ("Bot-Token" = [])),
    params(
        ("target" = Reference, Path),
    ),
    responses(
        (
            status = 200,
            description = "PNG image with the users default avatar.",
            content_type = "image/png",
            body = CachedFile,
        ),
    ),
)]
#[get("/<target>/default_avatar")]
pub async fn default_avatar(target: String) -> (ContentType, CachedFile) {
    (
        ContentType::PNG,
        CachedFile(match target.chars().last().unwrap() {
            '0' | '1' | '2' | '3' | 'S' | 'Z' => include_bytes!("avatars/2.png").to_vec(),
            '4' | '5' | '6' | '7' | 'T' => include_bytes!("avatars/3.png").to_vec(),
            '8' | '9' | 'A' | 'B' => include_bytes!("avatars/4.png").to_vec(),
            'C' | 'D' | 'E' | 'F' | 'V' => include_bytes!("avatars/5.png").to_vec(),
            'G' | 'H' | 'J' | 'K' | 'W' => include_bytes!("avatars/6.png").to_vec(),
            'M' | 'N' | 'P' | 'Q' | 'X' => include_bytes!("avatars/7.png").to_vec(),
            _ => include_bytes!("avatars/1.png").to_vec(),
        }),
    )
}
