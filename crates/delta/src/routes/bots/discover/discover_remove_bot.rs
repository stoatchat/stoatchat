use revolt_config::config;
use revolt_database::{
    util::reference::Reference, Database, DiscoverRequestStatus, DiscoverRequestType, User,
};

use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Delete Discover request
///
/// This cannot be used if your request is no longer in the queue (ie approved or rejected).
/// If you wish to reapply after a rejection, submit another POST.
/// This endpoint is ONLY USEFUL in production on stoat.chat/app .
#[openapi(tag = "Discover")]
#[delete("/<bot_id>/discover")]
pub async fn discover_remove_bot(
    db: &State<Database>,
    bot_id: Reference<'_>,
    user: User,
) -> Result<EmptyResponse> {
    let config = config().await;
    if !config.production {
        return Err(create_error!(NoEffect));
    }

    let bot = bot_id.as_bot(db).await?;
    if (bot.owner != user.id && bot.id != user.id) && !user.privileged {
        return Err(create_error!(NotOwner));
    }

    if db
        .get_discover_ban(DiscoverRequestType::Bot, &bot.id)
        .await
        .is_ok()
    {
        return Err(create_error!(Banned));
    }

    let ret = db
        .fetch_discover_request_by_item_id(DiscoverRequestType::Bot, &bot.id)
        .await?;

    match ret.status {
        DiscoverRequestStatus::Approved(_) => Err(create_error!(ContactSupport {
            locale: "discover.bot_removal_approved".to_string(),
            msg: "Contact support to have your bot removed from Discover".to_string()
        })),
        DiscoverRequestStatus::Removed(_) => Err(create_error!(ContactSupport {
            locale: "discover.bot_removal_removed".to_string(),
            msg: "Your bot has been removed from discover, contact support for more information"
                .to_string()
        })),
        DiscoverRequestStatus::Denied(_) => Err(create_error!(NoEffect)),
        DiscoverRequestStatus::Pending | DiscoverRequestStatus::UnderReview => {
            db.delete_discover_request(ret.request_type, &ret.request_id)
                .await?;
            Ok(EmptyResponse)
        }
    }
}
