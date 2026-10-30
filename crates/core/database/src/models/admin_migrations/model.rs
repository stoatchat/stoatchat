auto_derived!(
    /// Document representing migration information
    pub struct MigrationInfo {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: i32,
        /// Current database revision
        pub revision: i32,
    }
);

#[cfg(test)]
mod tests {
    use lapin::{ExchangeKind, options::ExchangeDeclareOptions, types::FieldTable};
    #[tokio::test]
    async fn migrate() {
        database_test!(|db| async move {
            // Initialise the database
            db.migrate_database().await.unwrap();

            // Migrate the existing database
            db.migrate_database().await.unwrap();

            // The required inbox indexes must exist after migration.
            #[cfg(feature = "mongodb")]
            if let crate::Database::MongoDb(db) = &db {
                use bson::doc;

                let result = db
                    .db()
                    .run_command(doc! { "listIndexes": "inbox" })
                    .await
                    .expect("list inbox indexes");

                let indexes = result
                    .get_document("cursor")
                    .expect("cursor")
                    .get_array("firstBatch")
                    .expect("firstBatch");

                for required in ["user_message", "message", "channel", "server"] {
                    let found = indexes.iter().any(|index| {
                        index.as_document().is_some_and(|index| {
                            index.get_str("name").ok() == Some(required)
                        })
                    });

                    assert!(found, "missing required inbox index {required}");
                }
            }
        });
    }
}
