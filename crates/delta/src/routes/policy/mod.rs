use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

mod acknowledge_policy_changes;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        // Policy
        acknowledge_policy_changes::acknowledge_policy_changes,
    ]
}
