use revolt_models::v0;

auto_derived_partial! {
    pub struct AdminComment {
        /// The comment ID
        #[serde(rename = "_id")]
        pub id: String,
        /// The Object this comment is attached to
        pub object_id: String,
        /// The ID of the case this comment is attached to, if applicable
        pub case_id: Option<String>,
        /// The author ID
        pub user_id: String,
        /// When the comment was edited, if applicable, in iso8601
        pub edited_at: Option<String>,
        /// The content, if not a system event
        pub content: Option<String>,
        /// The system event that occurred, if applicable
        pub system_message: Option<String>,
        /// The system message target
        pub system_message_target: Option<String>,
        /// The system message raw context
        pub system_message_context: Option<String>,
    },
    "PartialAdminComment"
}

impl AdminComment {
    pub fn new(
        object_id: &str,
        user_id: &str,
        content: &str,
        case_id: Option<&str>,
    ) -> AdminComment {
        let id = ulid::Ulid::new().to_string();
        AdminComment {
            id,
            object_id: object_id.to_string(),
            case_id: case_id.map(|c| c.to_string()),
            user_id: user_id.to_string(),
            edited_at: None,
            content: Some(content.to_string()),
            system_message: None,
            system_message_context: None,
            system_message_target: None,
        }
    }

    pub fn new_system_message(
        object_id: &str,
        user_id: &str,
        case_id: &str,
        system_message_kind: v0::AdminAuditItemActions,
        context: Option<&str>,
        target: &str,
    ) -> AdminComment {
        let id = ulid::Ulid::new().to_string();
        AdminComment {
            id,
            object_id: object_id.to_string(),
            case_id: Some(case_id.to_string()),
            user_id: user_id.to_string(),
            edited_at: None,
            content: None,
            system_message: Some(serde_json::to_string(&system_message_kind).unwrap()), // if this explodes i'll eat my hat.
            system_message_context: context.map(|c| c.to_string()),
            system_message_target: Some(target.to_string()),
        }
    }

    /// Edit the comment, updating the edited_at time as well
    pub fn edit(&mut self, content: &str) {
        self.content = Some(content.to_string());
        self.edited_at = Some(
            iso8601_timestamp::Timestamp::now_utc()
                .format_short()
                .to_string(),
        );
    }
}

impl PartialAdminComment {
    pub fn new() -> PartialAdminComment {
        PartialAdminComment::default()
    }
    /// Edit the comment, updating the edited_at time as well
    pub fn edit(&mut self, content: &str) {
        self.content = Some(content.to_string());
        self.edited_at = Some(
            iso8601_timestamp::Timestamp::now_utc()
                .format_short()
                .to_string(),
        );
    }
}
