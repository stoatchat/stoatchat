use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::Route;

mod accounts;
mod cases;
mod comments;
mod meta;
mod reports;
mod roles;
mod search;
mod servers;
mod users;

mod util;

pub fn routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        meta::create_user::admin_create_user,
        meta::edit_user::admin_edit_user,
        meta::fetch_users::admin_fetch_users,
        meta::create_token::admin_create_token,
        meta::revoke_token::admin_revoke_token,
        comments::comment_create::admin_comment_create,
        comments::comment_edit::admin_comment_edit,
        comments::comment_fetch_case::admin_comment_fetch_case,
        comments::comment_fetch_object::admin_comment_fetch_object,
        servers::fetch::server_get::admin_server_get,
        servers::fetch::server_get_participants::admin_server_get_participants,
        servers::fetch::server_get_all_members::admin_server_get_members,
        servers::actions::server_add_members::admin_server_add_member,
        servers::actions::server_ban_members::admin_server_ban_member,
        servers::actions::server_unban_members::admin_server_unban_member,
        servers::actions::server_change_owner::admin_server_change_owner,
        servers::actions::server_create_invite::admin_server_create_invite,
        servers::actions::server_delete_invites::admin_server_delete_invites,
        servers::actions::server_delete::admin_server_delete,
        servers::actions::server_edit::admin_server_edit,
        servers::actions::server_remove_members::admin_server_remove_members
    ]
}
