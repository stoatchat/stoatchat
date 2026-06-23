use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

mod webhook_delete;
mod webhook_delete_message;
mod webhook_delete_token;
mod webhook_edit;
mod webhook_edit_message;
mod webhook_edit_token;
mod webhook_execute;
mod webhook_execute_github;
mod webhook_fetch;
mod webhook_fetch_token;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        webhook_delete_message::webhook_delete_message,
        webhook_delete_token::webhook_delete_token,
        webhook_delete::webhook_delete,
        webhook_edit_message::webhook_edit_message,
        webhook_edit_token::webhook_edit_token,
        webhook_edit::webhook_edit,
        webhook_execute_github::webhook_execute_github,
        webhook_execute::webhook_execute,
        webhook_fetch_token::webhook_fetch_token,
        webhook_fetch::webhook_fetch,
    ]
}
