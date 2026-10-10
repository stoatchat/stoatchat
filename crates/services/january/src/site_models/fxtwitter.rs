use serde::Deserialize;

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterStatusFacets {
    pub r#type: String,
    pub indices: (usize, usize),
    display: String,
    original: String,
    replacement: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterStatusText {
    pub text: String,
    pub display_text_range: (usize, usize),
    pub facets: Vec<FxTwitterStatusFacets>,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterStatus {
    pub url: String,
    pub id: String,
    pub text: String,
    pub raw_text: FxTwitterStatusText,
    pub author: FxTwitterAuthor,
    pub media: FxTwitterMediaAll,

    pub replies: Option<usize>,
    pub reposts: Option<usize>,
    pub likes: Option<usize>,
    pub bookmarks: Option<usize>,
    pub quotes: Option<usize>,
    pub views: Option<usize>,
    pub created_timestamp: usize,

    pub quote: Option<FxTwitterQuote>,
}

impl FxTwitterStatus {
    /// The quoted status, if there is one and it is available
    pub fn quoted_status(&self) -> Option<&FxTwitterStatus> {
        match &self.quote {
            Some(FxTwitterQuote::Status(status)) => Some(status),
            _ => None,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FxTwitterQuote {
    Status(Box<FxTwitterStatus>),
    /// Placeholder for a quoted status that is deleted, suspended, private, etc.
    Tombstone(FxTwitterTombstone),
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterTombstone {
    pub reason: String,
    pub message: String,
    pub id: Option<String>,
    pub url: Option<String>,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterAuthor {
    pub screen_name: String,
    pub name: String,
    pub url: String,
    pub id: String,
    pub banner_url: Option<String>,
    pub avatar_url: Option<String>,
    pub joined: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterMediaPhotoElement {
    pub id: String,
    pub url: String,
    pub width: usize,
    pub height: usize,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterMediaVideoElementFormat {
    pub url: String,
    pub container: String,
    pub bitrate: Option<usize>,
    pub codec: Option<String>,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterMediaVideoElement {
    pub id: String,
    pub url: String,
    pub thumbnail_url: String,
    pub width: usize,
    pub height: usize,
    pub duration: usize,
    pub format: String,
    pub formats: Vec<FxTwitterMediaVideoElementFormat>,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FxTwitterMediaElement {
    #[serde(alias = "gif")]
    Video(FxTwitterMediaVideoElement),
    Photo(FxTwitterMediaPhotoElement),
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterMediaAll {
    pub all: Option<Vec<FxTwitterMediaElement>>,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct FxTwitterResult {
    pub status: FxTwitterStatus,
    pub author: FxTwitterAuthor,
}
