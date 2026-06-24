use revolt_result::Result;
use ulid::Ulid;

use crate::{events::client::EventV1, BotInformation, Database, PartialUser, User};

auto_derived_partial!(
    /// Bot
    pub struct Bot {
        /// Bot Id
        ///
        /// This equals the associated bot user's id.
        #[serde(rename = "_id")]
        pub id: String,
        /// User Id of the bot owner
        pub owner: String,
        /// Token used to authenticate requests for this bot
        pub token: String,
        /// Whether the bot is public
        /// (may be invited by anyone)
        pub public: bool,

        /// Whether to enable analytics
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub analytics: bool,
        /// Whether this bot should be publicly discoverable
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub discoverable: bool,
        /// Reserved; URL for handling interactions
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub interactions_url: String,
        /// URL for terms of service
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub terms_of_service_url: String,
        /// URL for privacy policy
        #[serde(skip_serializing_if = "String::is_empty", default)]
        pub privacy_policy_url: String,

        /// Enum of bot flags
        #[serde(skip_serializing_if = "Option::is_none")]
        pub flags: Option<i32>,
    },
    "PartialBot"
);

auto_derived!(
    /// Optional fields on bot object
    pub enum FieldsBot {
        Token,
        InteractionsURL,
    }
);

#[allow(clippy::derivable_impls)]
impl Default for Bot {
    fn default() -> Self {
        Self {
            id: Default::default(),
            owner: Default::default(),
            token: Default::default(),
            public: Default::default(),
            analytics: Default::default(),
            discoverable: Default::default(),
            interactions_url: Default::default(),
            terms_of_service_url: Default::default(),
            privacy_policy_url: Default::default(),
            flags: Default::default(),
        }
    }
}

#[allow(clippy::disallowed_methods)]
impl Bot {
    /// Create a new bot
    pub async fn create<D>(
        db: &Database,
        username: String,
        owner: &User,
        data: D,
    ) -> Result<(Bot, User)>
    where
        D: Into<Option<PartialBot>>,
    {
        if owner.bot.is_some() {
            return Err(create_error!(IsBot));
        }

        if db.get_number_of_bots_by_user(&owner.id).await? >= owner.limits().await.bots {
            return Err(create_error!(ReachedMaximumBots));
        }

        let id = Ulid::new().to_string();

        let user = User::create(
            db,
            username,
            Some(id.to_string()),
            Some(PartialUser {
                bot: Some(BotInformation {
                    owner: owner.id.to_string(),
                }),
                ..Default::default()
            }),
        )
        .await?;

        let mut bot = Bot {
            id,
            owner: owner.id.to_string(),
            token: nanoid::nanoid!(64),
            ..Default::default()
        };

        if let Some(data) = data.into() {
            bot.apply_options(data);
        }

        db.insert_bot(&bot).await?;
        Ok((bot, user))
    }

    /// Remove a field from this object
    pub fn remove_field(&mut self, field: &FieldsBot) {
        match field {
            FieldsBot::Token => self.token = nanoid::nanoid!(64),
            FieldsBot::InteractionsURL => {
                self.interactions_url = String::new();
            }
        }
    }

    /// Update this bot
    pub async fn update(
        &mut self,
        db: &Database,
        mut partial: PartialBot,
        remove: Vec<FieldsBot>,
    ) -> Result<()> {
        if remove.contains(&FieldsBot::Token) {
            partial.token = Some(nanoid::nanoid!(64));
        }

        for field in &remove {
            self.remove_field(field);
        }

        db.update_bot(&self.id, &partial, remove).await?;

        if partial.token.is_some() {
            EventV1::Logout.private(self.id.clone()).await;
        }

        self.apply_options(partial);
        Ok(())
    }

    /// Delete this bot
    pub async fn delete(&self, db: &Database) -> Result<()> {
        db.fetch_user(&self.id).await?.mark_deleted(db).await?;
        db.delete_bot(&self.id).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{Bot, FieldsBot, PartialBot, User};

    #[tokio::test]
    async fn crud() {
        database_test!(|db| async move {
            let owner = User::create(&db, "Owner".to_string(), None, None)
                .await
                .unwrap();

            let (bot, _) = Bot::create(
                &db,
                "Bot Name".to_string(),
                &owner,
                PartialBot {
                    token: Some("my token".to_string()),
                    interactions_url: Some("some url".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

            assert!(!bot.interactions_url.is_empty());

            let mut updated_bot = bot.clone();
            updated_bot
                .update(
                    &db,
                    PartialBot {
                        public: Some(true),
                        ..Default::default()
                    },
                    vec![FieldsBot::Token, FieldsBot::InteractionsURL],
                )
                .await
                .unwrap();

            let fetched_bot1 = db.fetch_bot(&bot.id).await.unwrap();
            let fetched_bot2 = db.fetch_bot_by_token(&fetched_bot1.token).await.unwrap();
            let fetched_bots = db.fetch_bots_by_user(&owner.id).await.unwrap();

            assert!(!bot.public);
            assert!(fetched_bot1.public);
            assert!(!bot.interactions_url.is_empty());
            assert!(fetched_bot1.interactions_url.is_empty());
            assert_ne!(bot.token, fetched_bot1.token);
            assert_eq!(updated_bot, fetched_bot1);
            assert_eq!(fetched_bot1, fetched_bot2);
            assert_eq!(fetched_bot1, fetched_bots[0]);
            assert_eq!(1, db.get_number_of_bots_by_user(&owner.id).await.unwrap());

            bot.delete(&db).await.unwrap();
            assert!(db.fetch_bot(&bot.id).await.is_err());
            assert_eq!(0, db.get_number_of_bots_by_user(&owner.id).await.unwrap());
            assert_eq!(db.fetch_user(&bot.id).await.unwrap().flags, Some(2))
        });
    }
}
