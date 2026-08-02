use super::File;

auto_derived!(
    /// Image positioning and size
    pub enum ImageSize {
        /// Show large preview at the bottom of the embed
        Large,
        /// Show small preview to the side of the embed
        Preview,
    }

    /// Image
    pub struct Image {
        /// URL to the original image
        pub url: String,
        /// Width of the image
        pub width: usize,
        /// Height of the image
        pub height: usize,
        /// Positioning and size
        pub size: ImageSize,
    }

    /// Video
    pub struct Video {
        /// URL to the original video
        pub url: String,
        /// Width of the video
        pub width: usize,
        /// Height of the video
        pub height: usize,
    }

    /// Type of remote Twitch content
    pub enum TwitchType {
        Channel,
        Video,
        Clip,
    }

    /// Type of remote Lightspeed.tv content
    pub enum LightspeedType {
        Channel,
    }

    /// Type of remote Bandcamp content
    pub enum BandcampType {
        Album,
        Track,
    }

    /// Information about special remote content
    #[serde(tag = "type")]
    pub enum Special {
        /// No remote content
        None,
        /// Content hint that this contains a GIF
        ///
        /// Use metadata to find video or image to play
        GIF,
        /// YouTube video
        YouTube {
            id: String,
            creator_name: Option<String>,
            creator_url: Option<String>,

            #[serde(skip_serializing_if = "Option::is_none")]
            timestamp: Option<String>,
        },
        /// Lightspeed.tv stream
        Lightspeed {
            content_type: LightspeedType,
            id: String,
        },
        /// Twitch stream or clip
        Twitch {
            content_type: TwitchType,
            id: String,
        },
        /// Spotify track
        Spotify { content_type: String, id: String },
        /// Soundcloud track
        Soundcloud,
        /// Bandcamp track
        Bandcamp {
            content_type: BandcampType,
            id: String,
        },
        AppleMusic {
            album_id: String,

            #[serde(skip_serializing_if = "Option::is_none")]
            track_id: Option<String>,
        },
        /// Streamable Video
        Streamable { id: String },
    }

    /// Website metadata
    pub struct WebsiteMetadata {
        /// Direct URL to web page
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        /// Original direct URL
        #[serde(skip_serializing_if = "Option::is_none")]
        pub original_url: Option<String>,
        /// Remote content
        #[serde(skip_serializing_if = "Option::is_none")]
        pub special: Option<Special>,

        /// Title of website
        #[serde(skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        /// Description of website
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        /// Embedded image
        #[serde(skip_serializing_if = "Option::is_none")]
        pub image: Option<Image>,
        /// Embedded video
        #[serde(skip_serializing_if = "Option::is_none")]
        pub video: Option<Video>,

        /// Site name
        #[serde(skip_serializing_if = "Option::is_none")]
        pub site_name: Option<String>,
        /// URL to site icon
        #[serde(skip_serializing_if = "Option::is_none")]
        pub icon_url: Option<String>,
        /// CSS Colour
        #[serde(skip_serializing_if = "Option::is_none")]
        pub colour: Option<String>,
    }

    /// Text Embed
    pub struct Text {
        /// URL to icon
        #[serde(skip_serializing_if = "Option::is_none")]
        pub icon_url: Option<String>,
        /// URL for title
        #[serde(skip_serializing_if = "Option::is_none")]
        pub url: Option<String>,
        /// Title of text embed
        #[serde(skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        /// Description of text embed
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        /// ID of uploaded autumn file
        #[serde(skip_serializing_if = "Option::is_none")]
        pub media: Option<File>,
        /// CSS Colour
        #[serde(skip_serializing_if = "Option::is_none")]
        pub colour: Option<String>,
    }

    /// Embed
    #[serde(tag = "type")]
    #[derive(Default)]
    pub enum Embed {
        Website(WebsiteMetadata),
        Image(Image),
        Video(Video),
        Text(Text),
        #[default]
        None,
    }
);

impl WebsiteMetadata {
    /// Truncate strings in metadata
    pub fn truncate(&mut self) {
        if let Some(s) = self.url.as_mut() {
            s.truncate(256);
        }

        if let Some(s) = self.original_url.as_mut() {
            s.truncate(256);
        }

        if let Some(s) = self.title.as_mut() {
            s.truncate(100);
        }

        if let Some(s) = self.description.as_mut() {
            s.truncate(1000);
        }

        if let Some(s) = self.site_name.as_mut() {
            s.truncate(32);
        }

        if let Some(s) = self.icon_url.as_mut() {
            s.truncate(256);
        }

        if let Some(s) = self.colour.as_mut() {
            s.truncate(32);
        }
    }

    /// Check if this is considered "empty"
    pub fn is_empty(&self) -> bool {
        (self.title.is_none() || self.title.as_ref().is_some_and(|f| f.is_empty()))
            && (self.description.is_none()
                || self.description.as_ref().is_some_and(|f| f.is_empty()))
            && self.special.is_none()
            && self.video.is_none()
            && self.image.is_none()
    }
}
