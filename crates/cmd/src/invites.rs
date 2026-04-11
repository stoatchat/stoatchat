use futures::TryStreamExt;
use mongodb::{Client, Collection};
use revolt_config::config;
use mongodb::{bson::{doc}};
use serde::{Serialize, Deserialize};
use rand::{Rng, rng};
use rand::distr::Alphanumeric;


use tabled::{builder::Builder};

#[derive(Serialize,Deserialize,Debug)]
struct Invite {
    _id: String,
    used: Option<bool>,
    claimed_by: Option<String>,
}

async fn get_invite_collection() -> Option<Collection<Invite>> {
    let config = config().await;

    let mongodb_client_result = Client::with_uri_str(config.database.mongodb).await;
    if mongodb_client_result.clone().is_err() {
        eprintln!("MongoDB connection failed: {:?}", mongodb_client_result.err());
        return None;
    }

    let client = mongodb_client_result.unwrap();
    let db = client.database("revolt");

    Some(db.collection("invites"))
}

pub async fn list_invites(unused_only: bool) -> std::process::ExitCode {
    eprintln!("Listing Invites");
    let database = revolt_database::DatabaseInfo::Auto.connect().await.unwrap();

    let invites_collection = get_invite_collection().await;
    if invites_collection.is_none() {
        panic!("Failed to connect to MongoDB");
    }

    let mut invites_cursor = invites_collection.unwrap().find(doc! {}).await;
    let mut builder = Builder::default();
    builder.push_record(vec! ["Invite Code", "Used", "Claimed By"]);
    while let Ok(Some(invite)) = invites_cursor.as_mut().expect("MongoDB cursor failure").try_next().await {
        if invite.used.is_some() && invite.used.unwrap() && invite.claimed_by.is_some() {
            if !unused_only {
                let claimed_by = database.fetch_user(invite.claimed_by.unwrap().as_str()).await;
                let mut claimed_by_name = "Unknown".to_string();
                if let Ok(claimed_by) = claimed_by {
                    claimed_by_name = format!("{}#{}", claimed_by.username, claimed_by.discriminator);
                }
                builder.push_record(vec![invite._id, "True".to_string(), claimed_by_name]);
            }
        }
        else {
            builder.push_record(vec![invite._id, "False".to_string(), "None".to_string()]);
        }
    }
    let mut table = builder.build();
    table.with(tabled::settings::Style::rounded());
    println!("{}", table);

    std::process::ExitCode::SUCCESS
}


pub async fn get_invite(invite_id: String) -> std::process::ExitCode {
    let database = revolt_database::DatabaseInfo::Auto.connect().await.unwrap();

    let invites_collection = get_invite_collection().await;
    if invites_collection.is_none() {
        panic!("Failed to connect to MongoDB");
    }

    let invite = invites_collection.unwrap().find_one(doc! {
        "_id": invite_id.clone(),
    }).await;

    if invite.is_err() {
        eprintln!("Error finding invite: {:?}", invite.err());
        return std::process::ExitCode::FAILURE;
    }
    let invite_contents = invite.unwrap();
    if invite_contents.is_some(){
        let mut builder = Builder::default();
        builder.push_record(vec! ["Invite Code", "Used", "Claimed By"]);

        let mut claimed_by_name = "-".to_string();
        let mut used = "False".to_string();

        let inv = invite_contents.unwrap();
        if inv.used.is_some_and(|i| i == true) {
            let claimed_user = database.fetch_user(inv.claimed_by.unwrap().as_str()).await;
            if claimed_user.is_ok() {
                let claimed_by = claimed_user.unwrap();
                claimed_by_name = format!("{}#{}", claimed_by.username, claimed_by.discriminator);
            } else {
                claimed_by_name = "Unknown".to_string();
            }
            used = "True".to_string();
        }
        builder.push_record(vec![inv._id, used, claimed_by_name]);
        let mut table = builder.build();
        table.with(tabled::settings::Style::rounded());
        println!("{}", table);

        return std::process::ExitCode::SUCCESS
    }

    eprintln!("Invite not found");
    std::process::ExitCode::SUCCESS
}

fn random_invite_code() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect()
}


pub async fn create_invite(invite_code: Option<String>) -> std::process::ExitCode {
    let invites_collection = get_invite_collection().await;
    if invites_collection.is_none() {
        panic!("Failed to connect to MongoDB");
    }

    let inv_code: String;
    if invite_code.is_some() {
        inv_code = invite_code.unwrap();
    } else {
        inv_code = random_invite_code();
    }

    let invite = invites_collection.unwrap().insert_one(Invite{_id: inv_code, used: None, claimed_by: None }).await;
    if invite.is_err() {
        eprintln!("Failed to add invite: {:?}", invite.err().unwrap());
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

pub async fn delete_invite(invite_code: String) -> std::process::ExitCode {
    let invites_collection = get_invite_collection().await;
    if invites_collection.is_none() {
        panic!("Failed to connect to MongoDB");
    }

    let invite = invites_collection.unwrap().delete_one(doc!{"_id": invite_code}).await;
    if invite.is_err() {
        eprintln!("Failed to delete invite: {:?}", invite.err().unwrap());
        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}