use revolt_result::{create_error, Error};
use revolt_rocket_okapi::{OpenApiError, gen::OpenApiGenerator, request::{OpenApiFromRequest, RequestHeaderInput}, revolt_okapi::openapi3::{Parameter, ParameterValue}};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use schemars::schema::{InstanceType, SchemaObject, SingleOrVec};

pub struct AuditLogReason(pub Option<String>);

#[async_trait]
impl<'r> FromRequest<'r> for AuditLogReason {
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let reason = req.headers().get_one("x-audit-log-reason");

        if reason.is_some_and(|str| str.len() > 512) {
            return Outcome::Error((Status::BadRequest, create_error!(PayloadTooLarge)));
        };

        Outcome::Success(Self(reason.map(|str| str.to_string())))
    }
}

impl OpenApiFromRequest<'_> for AuditLogReason {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> Result<RequestHeaderInput, OpenApiError> {
        Ok(RequestHeaderInput::Parameter(Parameter {
            name: "X-Audit-Log-Reason".to_string(),
            description: Some("Reason for action which is stored in the audit log.".to_string()),
            allow_empty_value: false,
            required: false,
            deprecated: false,
            extensions: schemars::Map::new(),
            location: "header".to_string(),
            value: ParameterValue::Schema {
                allow_reserved: false,
                example: None,
                examples: None,
                explode: None,
                style: None,
                schema: SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                    ..Default::default()
                },
            },
        }))
    }
}