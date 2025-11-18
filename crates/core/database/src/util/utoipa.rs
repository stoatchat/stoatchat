use utoipa::{
    openapi::{
        schema::SchemaType,
        security::{ApiKey, ApiKeyValue, SecurityScheme},
        ObjectBuilder, OpenApi, RefOr, Schema, Type,
    },
    Modify, PartialSchema, ToSchema,
};

use crate::util::reference::Reference;

pub struct TokenSecurity;

impl Modify for TokenSecurity {
    fn modify(&self, openapi: &mut OpenApi) {
        let components = openapi.components.get_or_insert_default();

        components.add_security_scheme(
            "Session-Token",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(
                "X-Session-Token".to_string(),
            ))),
        );

        components.add_security_scheme(
            "Bot-Token",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Bot-Ticket".to_string()))),
        );
    }
}

impl ToSchema for Reference<'_> {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Reference")
    }
}

impl PartialSchema for Reference<'_> {
    fn schema() -> RefOr<Schema> {
        RefOr::T(
            ObjectBuilder::new()
                .description(Some("An id referencing a stoat model."))
                .schema_type(SchemaType::Type(Type::String))
                .examples(["01FD58YK5W7QRV5H3D64KTQYX3"])
                .build()
                .into(),
        )
    }
}
