use axum::Json;

use crate::types;

/// Capture crate version from Cargo
static CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Root response from service
#[utoipa::path(
    get,
    path = "/",
    tag = "Misc",
    responses(
        (status = 200, description = "Root response", body = inline(types::RootResponse))
    )
)]
pub async fn root() -> Json<types::RootResponse<'static>> {
    Json(types::RootResponse {
        message: "Gifbox lives on!",
        version: CRATE_VERSION,
    })
}
