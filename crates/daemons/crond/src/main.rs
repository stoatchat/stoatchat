use revolt_config::configure;
use revolt_database::{DatabaseInfo, AMQP};
use revolt_result::Result;
use tasks::{acks, file_deletion, prune_dangling_files, prune_members};
use tokio::try_join;

pub mod tasks;

#[tokio::main]
async fn main() -> Result<()> {
    configure!(crond);

    let db = DatabaseInfo::Auto.connect().await.expect("database");
    let amqp = AMQP::new_auto().await;

    try_join!(
        file_deletion::task(db.clone()),
        prune_dangling_files::task(db.clone()),
        prune_members::task(db.clone()),
        acks::task(db.clone(), amqp.clone()),
    )
    .map(|_| ())
}
