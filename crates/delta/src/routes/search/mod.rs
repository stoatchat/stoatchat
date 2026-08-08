use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

mod search_messages;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        search_messages::search_messages
    ]
}
