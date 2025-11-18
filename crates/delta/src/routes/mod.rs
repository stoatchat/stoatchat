use revolt_config::Settings;
use revolt_database::util::{utoipa::TokenSecurity, reference::Reference};
use revolt_result::ErrorAddon;
pub use rocket::http::Status;
pub use rocket::response::Redirect;
use rocket::{Build, Rocket};
use rocket_authifier::SecurityAddon;
use utoipa::{
    openapi::{extensions::ExtensionsBuilder, OpenApi},
    Modify,
};

mod bots;
mod channels;
mod customisation;
mod invites;
mod onboard;
mod policy;
mod push;
mod root;
mod safety;
mod servers;
mod sync;
mod users;
mod webhooks;

pub fn mount(config: Settings, mut rocket: Rocket<Build>) -> Rocket<Build> {
    rocket = rocket
        .mount("/", routes![root::root])
        .mount("/users", users::routes())
        .mount("/bots", bots::routes())
        .mount("/channels", channels::routes())
        .mount("/servers", servers::routes())
        .mount("/invites", invites::routes())
        .mount("/custom", customisation::routes())
        .mount("/safety", safety::routes())
        .mount("/auth/account", rocket_authifier::routes::account::routes())
        .mount("/auth/session", rocket_authifier::routes::session::routes())
        .mount("/auth/mfa", rocket_authifier::routes::mfa::routes())
        .mount("/onboard", onboard::routes())
        .mount("/policy", policy::routes())
        .mount("/push", push::routes())
        .mount("/sync", sync::routes());

    if config.features.webhooks_enabled {
        rocket = rocket.mount("/webhooks", webhooks::routes());
    };

    rocket
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Stoat API",
        description = "Open source user-first chat platform.",
        terms_of_service = "https://stoat.chat/terms",
        contact(
            name = "Stoat Support",
            url = "https://stoat.chat",
            email = "contact@stoat.chat",
        ),
        license(
            name = "AGPLv3",
            url = "https://github.com/stoatchat/stoatchat/blob/master/LICENSE",
        ),
    ),
    servers(
        (
            description = "Stoat Production",
            url = "https://stoat.chat/api",
        ),
        (
            description = "Stoat Production",
            url = "https://api.stoat.chat"
        )
    ),
    external_docs(
        url = "https://developers.stoat.chat",
        description = "Revolt Developer Documentation",
    ),
    tags(
        (
            name = "Core",
            description = "Use in your applications to determine information about the Revolt node"
        ),
        (
            name = "User Information",
            description = "Query and fetch users on Revolt"
        ),
        (
            name = "Direct Messaging",
            description = "Direct message other users on Revolt"
        ),
        (
            name = "Relationships",
            description = "Manage your friendships and block list on the platform"
        ),
        (
            name = "Bots",
            description = "Create and edit bots"
        ),
        (
            name = "Channel Information",
            description = "Query and fetch channels on Revolt"
        ),
        (
            name = "Channel Invites",
            description = "Create and manage invites for channels"
        ),
        (
            name = "Channel Permissions",
            description = "Manage permissions for channels"
        ),
        (
            name = "Messaging",
            description = "Send and manipulate messages"
        ),
        (
            name = "Groups",
            description = "Create, invite users and manipulate groups"
        ),
        (
            name = "Voice",
            description = "Join and talk with other users"
        ),
        (
            name = "Server Information",
            description = "Query and fetch servers on Revolt"
        ),
        (
            name = "Server Members",
            description = "Find and edit server members"
        ),
        (
            name = "Server Permissions",
            description = "Manage permissions for servers"
        ),
        (
            name = "Invites",
            description = "View, join and delete invites"
        ),
        (
            name = "Account",
            description = "Manage your account"
        ),
        (
            name = "Session",
            description = "Create and manage sessions"
        ),
        (
            name = "MFA",
            description = "Multi-factor Authentication"
        ),
        (
            name = "Onboarding",
            description = "After signing up to Revolt, users must pick a unique username"
        ),
        (
            name = "Sync",
            description = "Upload and retrieve any JSON data between clients"
        ),
        (
            name = "Web Push",
            description = "Subscribe to and receive Revolt push notifications while offline"
        ),
        (
            name = "Webhooks",
            description = "Send messages from 3rd party services"
        ),
    ),
    paths(
        root::root,
    ),
    nest(
        (
            path = "/users",
            api = users::ApiDoc,
        ),
        (
            path = "/bots",
            api = bots::ApiDoc,
        ),
        (
            path = "/channels",
            api = channels::ApiDoc,
        ),
        (
            path = "/servers",
            api = servers::ApiDoc,
        ),
        (
            path = "/invites",
            api = invites::ApiDoc,
        ),
        (
            path = "/custom",
            api = customisation::ApiDoc,
        ),
        (
            path = "/safety",
            api = safety::ApiDoc,
        ),
        (
            path = "/auth/account",
            api = rocket_authifier::routes::account::ApiDoc,
        ),
        (
            path = "/auth/session",
            api = rocket_authifier::routes::session::ApiDoc,
        ),
        (
            path = "/auth/mfa",
            api = rocket_authifier::routes::mfa::ApiDoc,
        ),
        (
            path = "/onboard",
            api = onboard::ApiDoc,
        ),
        (
            path = "/policy",
            api = policy::ApiDoc,
        ),
        (
            path = "/push",
            api = push::ApiDoc,
        ),
        (
            path = "/sync",
            api = sync::ApiDoc,
        ),
        (
            path = "/webhooks",
            api = webhooks::ApiDoc,
        ),

    ),
    components(
        schemas(
            Reference,
        )
    ),
    modifiers(
        &Extensions,

        // TODO: merge these together when authifier is moved into the backend
        &SecurityAddon,
        &TokenSecurity,

        &ErrorAddon,
    ),
)]
pub struct ApiDoc;

struct Extensions;
impl Modify for Extensions {
    fn modify(&self, utoipa: &mut OpenApi) {
        utoipa.extensions = Some(
            ExtensionsBuilder::new()
                .add(
                    "tagGroups",
                    json!([
                      {
                        "name": "Revolt",
                        "tags": [
                          "Core"
                        ]
                      },
                      {
                        "name": "Users",
                        "tags": [
                          "User Information",
                          "Direct Messaging",
                          "Relationships"
                        ]
                      },
                      {
                        "name": "Bots",
                        "tags": [
                          "Bots"
                        ]
                      },
                      {
                        "name": "Channels",
                        "tags": [
                          "Channel Information",
                          "Channel Invites",
                          "Channel Permissions",
                          "Messaging",
                          "Interactions",
                          "Groups",
                          "Voice",
                          "Webhooks",
                        ]
                      },
                      {
                        "name": "Servers",
                        "tags": [
                          "Server Information",
                          "Server Members",
                          "Server Permissions"
                        ]
                      },
                      {
                        "name": "Invites",
                        "tags": [
                          "Invites"
                        ]
                      },
                      {
                        "name": "Customisation",
                        "tags": [
                          "Emojis"
                        ]
                      },
                      {
                        "name": "Platform Administration",
                        "tags": [
                          "Admin",
                          "User Safety"
                        ]
                      },
                      {
                        "name": "Authentication",
                        "tags": [
                          "Account",
                          "Session",
                          "Onboarding",
                          "MFA"
                        ]
                      },
                      {
                        "name": "Miscellaneous",
                        "tags": [
                          "Sync",
                          "Web Push"
                        ]
                      }
                    ]),
                )
                .build(),
        );
    }
}
