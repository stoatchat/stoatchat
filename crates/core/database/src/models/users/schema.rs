use revolt_okapi::openapi3::{SecurityScheme, SecuritySchemeData};
use revolt_rocket_okapi::{
    gen::OpenApiGenerator,
    request::{OpenApiFromRequest, RequestHeaderInput},
};

use crate::User;

impl OpenApiFromRequest<'_> for User {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        let mut requirements = schemars::Map::new();
        requirements.insert("Session Token".to_owned(), vec![]);

        Ok(RequestHeaderInput::Security(
            "Session Token".to_owned(),
            SecurityScheme {
                data: SecuritySchemeData::ApiKey {
                    name: "x-session-token".to_owned(),
                    location: "header".to_owned(),
                },
                description: Some("Used to authenticate as a user.".to_owned()),
                extensions: schemars::Map::new(),
            },
            requirements,
        ))
    }
}
