use crate::MongoDb;

use super::AbstractMigrations;

mod init;
mod scripts;

#[async_trait]
impl AbstractMigrations for MongoDb {
    #[cfg(test)]
    /// Drop the database
    async fn drop_database(&self) {
        self.db().drop().await.ok();
    }

    /// Migrate the database
    async fn migrate_database(&self) -> Result<(), ()> {
        info!("Migrating the database.");

        let list = self
            .list_database_names()
            .await
            .expect("Failed to fetch database names.");

        if list.iter().any(|x| x == &self.1) {
            scripts::migrate_database(self).await;
        } else {
            init::create_database(self).await;
        }

        let config = revolt_config::config().await;

        if config.features.admin_api_enabled {
            let db = self.db();
            let colls = db
                .list_collection_names()
                .await
                .expect("Failed to fetch collection names.");
            if !colls.iter().any(|x| x == "admin_audits") {
                info!("You've enabled the admin api for the first time. Setting up database...");
                init::create_admin_database(&db).await;
            }
        }

        Ok(())
    }
}
