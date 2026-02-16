use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct GuildTemplate {
    pub serialized_source_guild: Guild
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Guild {
    pub name: String,
    pub banner: Option<String>,
    pub roles: Vec<Role>,
    pub channels: Vec<Channel>,
    pub system_channel_id: Option<u32>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Role {
    pub id: u32,
    pub name: String,
    pub colors: Option<RoleColor>,
    pub hoist: bool,
    pub icon: Option<String>,
    pub position: Option<u32>,
    pub permissions: String,
    pub mentionable: bool
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct RoleColor {
    pub primary_color: u64,
    pub secondary_color: Option<u64>,
    pub tertiary_color: Option<u64>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Channel {
    pub id: u32,
    #[serde(rename = "type")]
    pub channel_type: u32,
    pub position: u32,
    pub permission_overwrites: Option<Vec<Overwrite>>,
    pub name: String,
    pub topic: Option<String>,
    pub nsfw: Option<bool>,
    pub user_limit: Option<u32>,
    pub parent_id: Option<u32>,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ChannelType {
    Text = 0,
    DM = 1,
    Voice = 2,
    Group = 3,
    Category = 4,
    Announcement = 5,
    AnnouncementThread = 10,
    PublicThread = 11,
    PrivateThread = 12,
    StageVoice = 13,
    Directory = 14,
    Forum = 15,
    Media = 16,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Overwrite {
    pub id: u32,
    #[serde(rename = "type")]
    pub overwrite_type: u32,
    pub allow: String,
    pub deny: String,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum OverwriteType {
    Role = 0,
    Member = 1,
}