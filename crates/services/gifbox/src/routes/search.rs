use axum::{
    extract::{Query, State},
    Json,
};
use revolt_database::User;
use revolt_result::{create_error, Result};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{giphy, types};

#[derive(Deserialize, IntoParams)]
pub struct SearchQueryParams {
    /// Search query
    #[param(example = "Wave")]
    pub query: String,
    /// Users locale
    #[param(example = "en_US")]
    pub locale: String,
    /// Amount of results to respond with
    pub limit: Option<u32>,
    /// Value of `next` for getting the next page of results with the current search query
    pub position: Option<String>,
}

/// Searches for GIFs with a query
#[utoipa::path(
    get,
    path = "/search",
    tag = "GIFs",
    security(("User Token" = []), ("Bot Token" = [])),
    params(SearchQueryParams),
    responses(
        (status = 200, description = "Search results", body = inline(types::PaginatedMediaResponse))
    )
)]
pub async fn search(
    _user: User,
    Query(params): Query<SearchQueryParams>,
    State(giphy): State<giphy::Giphy>,
) -> Result<Json<types::PaginatedMediaResponse>> {
    let offset: u64 = params
        .position
        .as_deref()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);

    giphy
        .search(
            &params.query,
            &params.locale,
            params.limit.unwrap_or(50),
            offset,
        )
        .await
        .map_err(|_| create_error!(InternalError))
        .map(|results| results.as_ref().clone().into())
        .map(Json)
}
