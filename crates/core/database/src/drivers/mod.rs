#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

use authifier::config::Captcha;
use authifier::config::EmailVerificationConfig;
use authifier::config::PasswordScanning;
use authifier::config::ResolveIp;
use authifier::config::SMTPSettings;
use authifier::config::Shield;
use authifier::config::Template;
use authifier::config::Templates;
use authifier::Authifier;
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
                    return Err("MongoDB not enabled.".to_string())
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
                        return Err("MongoDB not enabled.".to_string())
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

impl Database {
    /// Create an Authifier reference
    pub async fn to_authifier(self) -> Authifier {
        let config = config().await;

        let mut auth_config = authifier::Config {
            password_scanning: if config.api.security.easypwned.is_empty() {
                Default::default()
            } else {
                PasswordScanning::EasyPwned {
                    endpoint: config.api.security.easypwned,
                }
            },
            email_verification: if !config.api.smtp.host.is_empty() {
                EmailVerificationConfig::Enabled {
                    smtp: SMTPSettings {
                        from: config.api.smtp.from_address,
                        host: config.api.smtp.host,
                        username: config.api.smtp.username,
                        password: config.api.smtp.password,
                        reply_to: Some(
                            config
                                .api
                                .smtp
                                .reply_to
                                .unwrap_or("support@revolt.chat".into()),
                        ),
                        port: config.api.smtp.port,
                        use_tls: config.api.smtp.use_tls,
                        use_starttls: config.api.smtp.use_starttls,
                    },
                    expiry: Default::default(),
                    templates: if config.production {
                        Templates {
                            verify: Template {
                                title: "Verify your Revolt account.".into(),
                                text: include_str!("../../templates/verify.txt").into(),
                                url: format!("{}/login/verify/", config.hosts.app),
                                html: Some(include_str!("../../templates/verify.html").into()),
                            },
                            reset: Template {
                                title: "Reset your Revolt password.".into(),
                                text: include_str!("../../templates/reset.txt").into(),
                                url: format!("{}/login/reset/", config.hosts.app),
                                html: Some(include_str!("../../templates/reset.html").into()),
                            },
                            reset_existing: Template {
                                title: "You already have a Revolt account, reset your password."
                                    .into(),
                                text: include_str!("../../templates/reset-existing.txt").into(),
                                url: format!("{}/login/reset/", config.hosts.app),
                                html: Some(
                                    include_str!("../../templates/reset-existing.html").into(),
                                ),
                            },
                            deletion: Template {
                                title: "Confirm account deletion.".into(),
                                text: include_str!("../../templates/deletion.txt").into(),
                                url: format!("{}/delete/", config.hosts.app),
                                html: Some(include_str!("../../templates/deletion.html").into()),
                            },
                            welcome: None,
                        }
                    } else {
                        Templates {
                            verify: Template {
                                title: "Verify your account.".into(),
                                text: include_str!("../../templates/verify.whitelabel.txt").into(),
                                url: format!("{}/login/verify/", config.hosts.app),
                                html: None,
                            },
                            reset: Template {
                                title: "Reset your password.".into(),
                                text: include_str!("../../templates/reset.whitelabel.txt").into(),
                                url: format!("{}/login/reset/", config.hosts.app),
                                html: None,
                            },
                            reset_existing: Template {
                                title: "Reset your password.".into(),
                                text: include_str!("../../templates/reset.whitelabel.txt").into(),
                                url: format!("{}/login/reset/", config.hosts.app),
                                html: None,
                            },
                            deletion: Template {
                                title: "Confirm account deletion.".into(),
                                text: include_str!("../../templates/deletion.whitelabel.txt")
                                    .into(),
                                url: format!("{}/delete/", config.hosts.app),
                                html: None,
                            },
                            welcome: None,
                        }
                    },
                }
            } else {
                EmailVerificationConfig::Disabled
            },
            ..Default::default()
        };

        auth_config.invite_only = config.api.registration.invite_only;

        if !config.api.security.captcha.hcaptcha_key.is_empty() {
            auth_config.captcha = Captcha::HCaptcha {
                secret: config.api.security.captcha.hcaptcha_key,
            };
        }

        if !config.api.security.authifier_shield_key.is_empty() {
            auth_config.shield = Shield::Enabled {
                api_key: config.api.security.authifier_shield_key,
                strict: false,
            };
        }

        if config.api.security.trust_cloudflare {
            auth_config.resolve_ip = ResolveIp::Cloudflare;
        }

        Authifier {
            database: match self {
                Database::Reference(_) => Default::default(),
                #[cfg(feature = "mongodb")]
                Database::MongoDb(MongoDb(client, _)) => authifier::Database::MongoDb(
                    authifier::database::MongoDb(client.database("revolt")),
                ),
            },
            config: auth_config,
            #[cfg(feature = "tasks")]
            event_channel: Some(crate::tasks::authifier_relay::sender()),
            #[cfg(not(feature = "tasks"))]
            event_channel: None,
        }
    }
}
