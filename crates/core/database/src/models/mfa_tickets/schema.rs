use revolt_okapi::openapi3::{SecurityScheme, SecuritySchemeData};
use revolt_rocket_okapi::{
    gen::OpenApiGenerator,
    request::{OpenApiFromRequest, RequestHeaderInput},
};

use crate::{MFATicket, ValidatedTicket, UnvalidatedTicket};


impl<'r> OpenApiFromRequest<'r> for MFATicket {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        let mut requirements = schemars::Map::new();
        requirements.insert("MFA Ticket".to_owned(), vec![]);

        Ok(RequestHeaderInput::Security(
            "MFA Ticket".to_owned(),
            SecurityScheme {
                data: SecuritySchemeData::ApiKey {
                    name: "x-mfa-ticket".to_owned(),
                    location: "header".to_owned(),
                },
                description: Some("Used to authorise a request.".to_owned()),
                extensions: schemars::Map::new(),
            },
            requirements,
        ))
    }
}

impl<'r> OpenApiFromRequest<'r> for ValidatedTicket {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        let mut requirements = schemars::Map::new();
        requirements.insert("Valid MFA Ticket".to_owned(), vec![]);

        Ok(RequestHeaderInput::Security(
            "Valid MFA Ticket".to_owned(),
            SecurityScheme {
                data: SecuritySchemeData::ApiKey {
                    name: "x-mfa-ticket".to_owned(),
                    location: "header".to_owned(),
                },
                description: Some("Used to authorise a request.".to_owned()),
                extensions: schemars::Map::new(),
            },
            requirements,
        ))
    }
}

impl<'r> OpenApiFromRequest<'r> for UnvalidatedTicket {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        let mut requirements = schemars::Map::new();
        requirements.insert("Unvalidated MFA Ticket".to_owned(), vec![]);

        Ok(RequestHeaderInput::Security(
            "Unvalidated MFA Ticket".to_owned(),
            SecurityScheme {
                data: SecuritySchemeData::ApiKey {
                    name: "x-mfa-ticket".to_owned(),
                    location: "header".to_owned(),
                },
                description: Some("Used to authorise a request.".to_owned()),
                extensions: schemars::Map::new(),
            },
            requirements,
        ))
    }
}