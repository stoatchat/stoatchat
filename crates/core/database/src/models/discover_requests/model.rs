auto_derived!(
    #[derive(Hash)]
    pub enum DiscoverRequestType {
        Bot,
        Server,
    }

    pub enum DiscoverRequestStatus {
        Pending,
        UnderReview,
        Removed(Option<String>),
        Denied(Option<String>),   // reason
        Approved(Option<String>), // reason
    }

    /// Discover request
    pub struct DiscoverRequest {
        /// The type of request.
        pub request_type: DiscoverRequestType,
        /// The ID of the bot/server
        pub request_id: String,
        /// status of the request
        pub status: DiscoverRequestStatus,
    }

    pub struct DiscoverBan {
        /// Ban Id
        #[serde(rename = "_id")]
        pub id: String,
        /// The type of item.
        #[serde(rename = "type")]
        pub item_type: DiscoverRequestType,
        /// The ID of the bot/server
        pub item_id: String,
    }
);
