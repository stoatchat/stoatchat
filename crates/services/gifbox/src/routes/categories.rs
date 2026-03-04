use axum::{
    extract::State,
    Json,
};
use revolt_database::User;
use revolt_result::{create_error, Result};

use crate::{giphy, types};

/// Trending GIF categories
#[utoipa::path(
    get,
    path = "/categories",
    tag = "GIFs",
    security(("User Token" = []), ("Bot Token" = [])),
    responses(
        (status = 200, description = "Categories results", body = inline(Vec<types::CategoryResponse>))
    )
)]
pub async fn categories(
    _user: User,
    State(giphy): State<giphy::Giphy>,
) -> Result<Json<Vec<types::CategoryResponse>>> {
    giphy
        .categories()
        .await
        .map_err(|_| create_error!(InternalError))
        .map(|results| {
            (*results)
                .clone()
                .data
                .into_iter()
                .map(|cat| cat.into())
                .collect()
        })
        .map(Json)
}
