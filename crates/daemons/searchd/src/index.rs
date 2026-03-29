use revolt_config::capture_error;
use revolt_database::Database;
use revolt_search::ElasticsearchClient;

pub async fn index_existing_messages(db: Database, client: ElasticsearchClient) {
    log::info!("Starting bulk indexing.");

    let mut generator = db
        .fetch_all_messages()
        .await
        .expect("Database query failed");

    let mut chunk = Vec::new();

    while let Some(message) = generator.next().await {
        chunk.push(message);

        if chunk.len() >= 1000
            && let Err(e) = client
                .bulk_index_messages(&db, std::mem::take(&mut chunk))
                .await
        {
            log::error!("Error bulk indexing messages: {e}");
            capture_error(&e);
        }
    }

    if !chunk.is_empty()
        && let Err(e) = client.bulk_index_messages(&db, chunk).await
    {
        log::error!("Error bulk indexing messages: {e}");
        capture_error(&e);
    }

    log::info!("Finished bulk indexing.")
}
