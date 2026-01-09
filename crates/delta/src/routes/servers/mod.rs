use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

mod audit_log_query;
mod ban_create;
mod ban_list;
mod ban_remove;
mod channel_create;
mod emoji_list;
mod invites_fetch;
mod member_edit;
mod member_experimental_query;
mod member_fetch;
mod member_fetch_all;
mod member_remove;
mod permissions_set;
mod permissions_set_default;
mod roles_create;
mod roles_delete;
mod roles_edit;
mod roles_edit_positions;
mod roles_fetch;
mod server_ack;
mod server_create;
mod server_delete;
mod server_edit;
mod server_fetch;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        server_create::create_server,
        server_delete::delete,
        server_fetch::fetch,
        server_edit::edit,
        server_ack::ack,
        channel_create::create_server_channel,
        member_fetch_all::fetch_all,
        member_remove::kick,
        member_fetch::fetch,
        member_edit::edit,
        member_experimental_query::member_experimental_query,
        ban_create::ban,
        ban_remove::unban,
        ban_list::list,
        invites_fetch::invites,
        roles_create::create,
        roles_edit::edit,
        roles_fetch::fetch,
        roles_delete::delete,
        permissions_set::set_role_permission,
        permissions_set_default::set_default_server_permissions,
        emoji_list::list_emoji,
        roles_edit_positions::edit_role_ranks,
        audit_log_query::query,
    ]
}
