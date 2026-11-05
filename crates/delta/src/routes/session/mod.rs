use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

pub mod edit;
pub mod fetch_all;
pub mod login;
pub mod login_passkey;
pub mod logout;
pub mod revoke;
pub mod revoke_all;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        login::login,
        login_passkey::login_passkey,
        logout::logout,
        fetch_all::fetch_all,
        revoke::revoke,
        revoke_all::revoke_all,
        edit::edit
    ]
}
