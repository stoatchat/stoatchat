use super::{File, MemberCompositeKey, User};

#[cfg(feature = "validator")]
use validator::Validate;

auto_derived!(
    /// Server Ban
    pub struct ServerBan {
        /// Unique member id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: MemberCompositeKey,
        /// Reason for ban creation
        pub reason: Option<String>,
    }

    /// Information for new server ban
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataBanCreate {
        /// Ban reason
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 1024)))]
        pub reason: Option<String>,
        /// Messages to delete in seconds
        #[cfg_attr(feature = "validator", validate(range(min = 0, max = 604800)))]
        pub delete_message_seconds: Option<i64>,
    }

    /// Just enough information to list a ban
    pub struct BannedUser {
        /// Id of the banned user
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,
        /// Username of the banned user
        pub username: String,
        /// Discriminator of the banned user
        pub discriminator: String,
        /// Avatar of the banned user
        pub avatar: Option<File>,
    }

    /// Ban list result
    pub struct BanListResult {
        /// Users objects
        pub users: Vec<BannedUser>,
        /// Ban objects
        pub bans: Vec<ServerBan>,
    }
);

impl From<User> for BannedUser {
    fn from(user: User) -> Self {
        BannedUser {
            id: user.id,
            username: user.username,
            discriminator: user.discriminator,
            avatar: user.avatar,
        }
    }
}
