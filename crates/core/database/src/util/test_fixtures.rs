use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde_json::from_str;

use crate::{Channel, Database, Member, Server, User};

static RE_ID: Lazy<Regex> = Lazy::new(|| Regex::new("__ID:(\\d+)__").unwrap());

#[derive(Debug, Deserialize)]
#[serde(tag = "_object_type")]
enum LoadedFixture {
    User(User),
    Channel(Channel),
    Server(Server),
    ServerMember(Member),
}

pub async fn load_fixture(db: &Database, input: &str) -> HashMap<String, String> {
    let mut ids = HashMap::<String, String>::new();
    let input = RE_ID.replace_all(input, |cap: &Captures| {
        let d = cap.get(1).unwrap().as_str();

        if !ids.contains_key(d) {
            ids.insert(d.to_string(), ulid::Ulid::new().to_string());
        }

        ids.get(d).unwrap().clone()
    });

    // Deserialise the fixtures
    let items: Vec<LoadedFixture> = from_str(&input).expect("Failed to deserialise fixture");

    // Load all of the items within
    for item in items {
        #[allow(clippy::disallowed_methods)]
        match item {
            LoadedFixture::User(user) => db.insert_user(&user).await.unwrap(),
            LoadedFixture::Channel(channel) => db.insert_channel(&channel).await.unwrap(),
            LoadedFixture::Server(server) => db.insert_server(&server).await.unwrap(),
            LoadedFixture::ServerMember(member) => {
                db.insert_or_merge_member(&member).await.unwrap();
            }
        }
    }

    // Return IDs for ease of use
    ids
}

#[async_trait]
pub trait FetchFixture {
    async fn user(&self, db: &Database, d: usize) -> User;
    async fn channel(&self, db: &Database, d: usize) -> Channel;
    async fn server(&self, db: &Database, d: usize) -> Server;
    async fn member(&self, db: &Database, d_server: usize, d_user: usize) -> Member;
}

#[async_trait]
impl FetchFixture for HashMap<String, String> {
    async fn user(&self, db: &Database, d: usize) -> User {
        db.fetch_user(self.get(&d.to_string()).unwrap())
            .await
            .unwrap()
    }

    async fn channel(&self, db: &Database, d: usize) -> Channel {
        db.fetch_channel(self.get(&d.to_string()).unwrap())
            .await
            .unwrap()
    }

    async fn server(&self, db: &Database, d: usize) -> Server {
        db.fetch_server(self.get(&d.to_string()).unwrap())
            .await
            .unwrap()
    }

    async fn member(&self, db: &Database, d_server: usize, d_user: usize) -> Member {
        db.fetch_member(
            self.get(&d_server.to_string()).unwrap(),
            self.get(&d_user.to_string()).unwrap(),
        )
        .await
        .unwrap()
    }
}

#[macro_export]
macro_rules! fixture {
    ( $database:expr, $name:expr, $( $variable:ident $type:ident $id: expr )+ ) => {
        use $crate::util::test_fixtures::FetchFixture;

        let fixtures = $crate::util::test_fixtures::load_fixture(
            &$database,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/", $name, ".json")),
        )
        .await;

        $(
            let $variable = fixtures.$type(&$database, $id).await;
        )+
    };
}
