use crate::Database;
use revolt_result::Result;

/// Formats a user's name depending on their optional features and location.
/// Factors in server display names and user display names before falling back to username#discriminator.
/// Passing a server in which the user is not a member will result in an Err.
pub async fn format_display_name(
    db: &Database,
    user_id: &str,
    server_id: Option<&str>,
) -> Result<String> {
    if let Some(server_id) = server_id {
        let member = db.fetch_member(server_id, user_id).await?;
        if let Some(nick) = member.nickname {
            return Ok(nick);
        }
    }

    let user = db.fetch_user(user_id).await?;
    if let Some(display) = user.display_name {
        return Ok(display);
    }
    Ok(format!("{}#{}", user.username, user.discriminator))
}
