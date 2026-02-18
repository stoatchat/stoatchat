use once_cell::sync::Lazy;
use regex::Regex;
use revolt_database::{Database, User, Account};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Regex for valid usernames
///
/// Block zero width space
/// Block lookalike characters
pub static RE_USERNAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\p{L}|[\d_.-])+$").unwrap());

/// # Username Information
#[derive(Validate, Serialize, Deserialize, JsonSchema)]
pub struct DataChangeUsername {
    /// New username
    #[validate(length(min = 2, max = 32), regex = "RE_USERNAME")]
    username: String,
    /// Current account password
    #[validate(length(min = 8, max = 1024))]
    password: String,
}

/// # Change Username
///
/// Change your username.
#[openapi(tag = "User Information")]
#[patch("/@me/username", data = "<data>")]
pub async fn change_username(
    db: &State<Database>,
    account: Account,
    mut user: User,
    data: Json<DataChangeUsername>,
) -> Result<Json<v0::User>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    account
        .verify_password(&data.password)
        .map_err(|_| create_error!(InvalidCredentials))?;

    user.update_username(db, data.username).await?;
    Ok(Json(user.into(db, None).await))
}
