use revolt_config::config;

use mongodb::{Client};

pub async fn do_mongo_check() -> std::process::ExitCode {
    eprintln!("Starting MongoDB Health Check");
    let config = config().await;

    let mongodb_client_result = Client::with_uri_str(config.database.mongodb).await;
    if mongodb_client_result.clone().is_err() {
        eprintln!("MongoDB connection failed: {:?}", mongodb_client_result.err());
        return std::process::ExitCode::FAILURE;
    }

    let client = mongodb_client_result.unwrap();
    let databases_result = client.list_database_names().await;

    if databases_result.clone().is_err() {
        eprintln!("Database list failed: {:?}", databases_result.err());
        return std::process::ExitCode::FAILURE;
    }

    eprintln!("Successfully connected to MongoDB");
    std::process::ExitCode::SUCCESS
}