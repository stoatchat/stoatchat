use std::{future::Future, panic::AssertUnwindSafe, time::Duration};

use revolt_config::{capture_error, configure};
use revolt_database::{Database, DatabaseInfo};
use revolt_result::Result;
use tasks::*;
use tokio::{time::sleep, join};
use futures::FutureExt;

pub mod tasks;

pub async fn cron_task_wrapper<Fut: Future<Output = Result<()>>>(func: fn(Database) -> Fut, db: Database) {
    loop {
        let wrapper = AssertUnwindSafe(func(db.clone()));

        match wrapper.catch_unwind().await {
            Ok(Ok(())) => {
                log::error!("cron unexpectedly finshed, Retrying after 60s");
            },
            Ok(Err(error)) => {
                log::error!("cron task failed unexpectedly: {error:?}\nRetrying after 60s");
                capture_error(&error);
            },
            _ => {
                log::error!("cron task failed unexpectedly\nRetrying after 60s");
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

#[tokio::main]
async fn main() {
    configure!(crond);

    let db = DatabaseInfo::Auto.connect().await.expect("database");

    join!(
        cron_task_wrapper(file_deletion::task, db.clone()),
        cron_task_wrapper(prune_dangling_files::task, db.clone()),
        cron_task_wrapper(prune_members::task, db.clone()),
        cron_task_wrapper(delete_accounts::task, db.clone()),
    );
}
