#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;


use rand::Rng;
use revolt_config::config;

#[cfg(feature = "mongodb")]
pub use self::mongodb::*;
pub use self::reference::*;

/// Database information to use to create a client
pub enum DatabaseInfo {
    /// Auto-detect the database in use
    Auto,
    /// Auto-detect the database in use and create an empty testing database
    Test(String),
    /// Use the mock database
    Reference,
    /// Connect to MongoDB
    #[cfg(feature = "mongodb")]
    MongoDb { uri: String, database_name: String },
    /// Use existing MongoDB connection
    #[cfg(feature = "mongodb")]
    MongoDbFromClient(::mongodb::Client, String),
}

/// Database
#[derive(Clone, Debug)]
pub enum Database {
    /// Mock database
    Reference(ReferenceDb),
    /// MongoDB database
    #[cfg(feature = "mongodb")]
    MongoDb(MongoDb),
}

impl DatabaseInfo {
    /// Create a database client from the given database information
    #[async_recursion]
    pub async fn connect(self) -> Result<Database, String> {
        let config = config().await;

        match self {
            DatabaseInfo::Auto => {
                if std::env::var("TEST_DB").is_ok() {
                    DatabaseInfo::Test(format!(
                        "revolt_test_{}",
                        rand::thread_rng().gen_range(1_000_000..10_000_000)
                    ))
                    .connect()
                    .await
                } else if !config.database.mongodb.is_empty() {
                    #[cfg(feature = "mongodb")]
                    return DatabaseInfo::MongoDb {
                        uri: config.database.mongodb,
                        database_name: "revolt".to_string(),
                    }
                    .connect()
                    .await;

                    #[cfg(not(feature = "mongodb"))]
                    return Err("MongoDB not enabled.".to_string());
                } else {
                    DatabaseInfo::Reference.connect().await
                }
            }
            DatabaseInfo::Test(database_name) => {
                match std::env::var("TEST_DB")
                    .expect("`TEST_DB` environment variable should be set to REFERENCE or MONGODB")
                    .as_str()
                {
                    "REFERENCE" => DatabaseInfo::Reference.connect().await,
                    "MONGODB" => {
                        #[cfg(feature = "mongodb")]
                        return DatabaseInfo::MongoDb {
                            uri: config.database.mongodb,
                            database_name,
                        }
                        .connect()
                        .await;

                        #[cfg(not(feature = "mongodb"))]
                        return Err("MongoDB not enabled.".to_string());
                    }
                    _ => unreachable!("must specify REFERENCE or MONGODB"),
                }
            }
            DatabaseInfo::Reference => Ok(Database::Reference(Default::default())),
            #[cfg(feature = "mongodb")]
            DatabaseInfo::MongoDb { uri, database_name } => {
                let client = ::mongodb::Client::with_uri_str(uri)
                    .await
                    .map_err(|_| "Failed to init db connection.".to_string())?;

                Ok(Database::MongoDb(MongoDb(client, database_name)))
            }
            #[cfg(feature = "mongodb")]
            DatabaseInfo::MongoDbFromClient(client, database_name) => {
                Ok(Database::MongoDb(MongoDb(client, database_name)))
            }
        }
    }
}
