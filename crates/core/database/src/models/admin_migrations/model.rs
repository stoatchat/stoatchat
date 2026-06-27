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
    #[tokio::test]
    async fn migrate() {
        database_test!(|db| async move {
            // Initialise the database
            db.migrate_database().await.unwrap();

            // Migrate the existing database
            db.migrate_database().await.unwrap();
        });
    }
}
