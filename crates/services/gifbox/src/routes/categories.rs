use axum::{
    extract::{Query, State},
    Json,
};
use revolt_database::User;
use revolt_result::{create_error, Result};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{tenor, types};

#[derive(Deserialize, IntoParams)]
pub struct CategoriesQueryParams {
    /// Users locale
    #[param(example = "en_US")]
    pub locale: String,
}

/// Trending GIF categories
#[utoipa::path(
    get,
    path = "/categories",
    tag = "GIFs",
    security(("User Token" = []), ("Bot Token" = [])),
    params(CategoriesQueryParams),
    responses(
        (status = 200, description = "Categories results", body = inline(Vec<types::CategoryResponse>))
    )
)]
pub async fn categories(
    _user: User,
    Query(params): Query<CategoriesQueryParams>,
    State(tenor): State<tenor::Tenor>,
) -> Result<Json<Vec<types::CategoryResponse>>> {
    tenor
        .categories(&params.locale)
        .await
        .map_err(|_| create_error!(InternalError))
        .map(|results| {
            (*results)
                .clone()
                .tags
                .into_iter()
                .map(|cat| cat.into())
                .collect()
        })
        .map(Json)
}
