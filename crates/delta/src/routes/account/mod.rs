use rocket::Route;
use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;

pub mod change_email;
pub mod change_password;
pub mod confirm_deletion;
pub mod create_account;
pub mod delete_account;
pub mod disable_account;
pub mod fetch_account;
pub mod password_reset;
pub mod resend_verification;
pub mod send_password_reset;
pub mod verify_email;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        create_account::create_account,
        resend_verification::resend_verification,
        confirm_deletion::confirm_deletion,
        fetch_account::fetch_account,
        delete_account::delete_account,
        disable_account::disable_account,
        change_password::change_password,
        change_email::change_email,
        verify_email::verify_email,
        password_reset::password_reset,
        send_password_reset::send_password_reset
    ]
}
