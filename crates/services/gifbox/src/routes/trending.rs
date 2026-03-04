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
pub struct TrendingQueryParams {
    #[param(example = "en_US")]
    /// Users locale
    pub locale: String,
    /// Amount of results to respond with
    pub limit: Option<u32>,
    /// Value of `next` for getting the next page of results of featured gifs
    pub position: Option<String>,
}

/// Trending GIFs
#[utoipa::path(
    get,
    path = "/featured",
    tag = "GIFs",
    security(("User Token" = []), ("Bot Token" = [])),
    params(TrendingQueryParams),
    responses(
        (status = 200, description = "Trending results", body = inline(types::PaginatedMediaResponse))
    )
)]
pub async fn trending(
    _user: User,
    Query(params): Query<TrendingQueryParams>,
    State(giphy): State<giphy::Giphy>,
) -> Result<Json<types::PaginatedMediaResponse>> {
    let offset: u64 = params
        .position
        .as_deref()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);

    giphy
        .trending(
            params.limit.unwrap_or(50),
            offset,
        )
        .await
        .map_err(|_| create_error!(InternalError))
        .map(|results| results.as_ref().clone().into())
        .map(Json)
}
