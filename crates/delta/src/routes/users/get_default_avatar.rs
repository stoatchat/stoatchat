use revolt_rocket_okapi::revolt_okapi::openapi3::{self, MediaType, RefOr};
use rocket::http::ContentType;
use rocket::response::{self, Responder};
use rocket::{Request, Response};
use schemars::schema::{InstanceType, SchemaObject, SingleOrVec};

pub struct CachedFile((ContentType, Vec<u8>));

pub static CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

impl<'r> Responder<'r, 'static> for CachedFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        Response::build_from(self.0.respond_to(req)?)
            .raw_header("Cache-Control", CACHE_CONTROL)
            .ok()
    }
}

impl revolt_rocket_okapi::response::OpenApiResponderInner for CachedFile {
    fn responses(
        _gen: &mut revolt_rocket_okapi::r#gen::OpenApiGenerator,
    ) -> std::result::Result<openapi3::Responses, revolt_rocket_okapi::OpenApiError> {
        let mut responses = schemars::Map::new();
        let mut content = schemars::Map::new();

        content.insert(
            "image/png".to_owned(),
            MediaType {
                schema: Some(SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                    format: Some("binary".to_owned()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        responses.insert(
            "200".to_string(),
            RefOr::Object(openapi3::Response {
                description: "Default Avatar Picture".to_string(),
                content,
                ..Default::default()
            }),
        );

        Ok(openapi3::Responses {
            responses,
            ..Default::default()
        })
    }
}

/// # Fetch Default Avatar
///
/// This returns a default avatar based on the given id.
#[openapi(tag = "User Information")]
#[get("/<target>/default_avatar")]
pub async fn default_avatar(target: String) -> CachedFile {
    CachedFile((
        ContentType::PNG,
        match target.chars().last().unwrap() {
            '0' | '1' | '2' | '3' | 'S' | 'Z' => include_bytes!("avatars/2.png").to_vec(),
            '4' | '5' | '6' | '7' | 'T' => include_bytes!("avatars/3.png").to_vec(),
            '8' | '9' | 'A' | 'B' => include_bytes!("avatars/4.png").to_vec(),
            'C' | 'D' | 'E' | 'F' | 'V' => include_bytes!("avatars/5.png").to_vec(),
            'G' | 'H' | 'J' | 'K' | 'W' => include_bytes!("avatars/6.png").to_vec(),
            'M' | 'N' | 'P' | 'Q' | 'X' => include_bytes!("avatars/7.png").to_vec(),
            _ => include_bytes!("avatars/1.png").to_vec(),
        },
    ))
}
