use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

pub mod create_ticket;
pub mod fetch_recovery;
pub mod fetch_status;
pub mod generate_recovery;
pub mod get_mfa_methods;
pub mod passkey_challenge;
pub mod passkey_delete;
pub mod passkey_edit;
pub mod passkey_list;
pub mod passkey_register_finish;
pub mod passkey_register_start;
pub mod totp_disable;
pub mod totp_enable;
pub mod totp_generate_secret;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        create_ticket::create_ticket,
        fetch_status::fetch_status,
        fetch_recovery::fetch_recovery,
        generate_recovery::generate_recovery,
        get_mfa_methods::get_mfa_methods,
        passkey_challenge::passkey_challenge,
        passkey_delete::passkey_delete,
        passkey_edit::passkey_edit,
        passkey_list::passkey_list,
        passkey_register_finish::passkey_register_finish,
        passkey_register_start::passkey_register_start,
        totp_disable::totp_disable,
        totp_enable::totp_enable,
        totp_generate_secret::totp_generate_secret,
    ]
}
