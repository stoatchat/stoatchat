use std::{future::Future, panic::AssertUnwindSafe, time::Duration};

use futures::FutureExt;
use revolt_database::{Database, DatabaseInfo, AMQP};
use revolt_result::Result;
use tasks::*;
use tokio::{join, time::sleep};
use tracing::{error, field::Empty, info, info_span, Instrument};

pub mod tasks;

pub async fn cron_task_wrapper<Fut: Future<Output = Result<()>>>(
    name: &'static str,
    func: fn(Database, AMQP) -> Fut,
    db: Database,
    amqp: AMQP,
) {
    loop {
        let wrapper = AssertUnwindSafe(func(db.clone(), amqp.clone()));

        match wrapper.catch_unwind().await {
            Ok(Ok(())) => {
                error!(task = name, "cron unexpectedly finished, retrying after 60s");
            }
            Ok(Err(error)) => {
                error!(task = name, ?error, "cron task failed unexpectedly, retrying after 60s");
            }
            Err(_) => {
                // the panic hook installed by stoat_otel has already logged the panic itself
                error!(task = name, "cron task panicked, retrying after 60s");
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

/// Run one iteration of a cron task as its own trace.
/// Tasks loop forever, so a span around the whole task would never close and never be exported
pub async fn traced_run<T, Fut: Future<Output = Result<T>>>(
    task: &'static str,
    run: Fut,
) -> Result<T> {
    let span = info_span!(
        parent: None,
        "cron.run",
        otel.name = format!("cron {task}"),
        task,
        otel.status_code = Empty,
        otel.status_message = Empty,
    );

    let result = run.instrument(span.clone()).await;

    if let Err(error) = &result {
        span.record("otel.status_code", "error");
        span.record("otel.status_message", format!("{error:?}"));
    }

    result
}

#[tokio::main]
async fn main() {
    let _telemetry = stoat_otel::init(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!(version = env!("CARGO_PKG_VERSION"), "Starting crond");

    revolt_config::config().await;

    let db = DatabaseInfo::Auto.connect().await.expect("database");
    let amqp = AMQP::new_auto().await;

    join!(
        cron_task_wrapper("file_deletion", file_deletion::task, db.clone(), amqp.clone()),
        cron_task_wrapper("prune_dangling_files", prune_dangling_files::task, db.clone(), amqp.clone()),
        cron_task_wrapper("prune_members", prune_members::task, db.clone(), amqp.clone()),
        cron_task_wrapper("delete_accounts", delete_accounts::task, db.clone(), amqp.clone()),
        cron_task_wrapper("acks", acks::task, db.clone(), amqp.clone()),
        cron_task_wrapper("delete_expired_invites", delete_expired_invites::task, db.clone(), amqp.clone()),
    );
}
