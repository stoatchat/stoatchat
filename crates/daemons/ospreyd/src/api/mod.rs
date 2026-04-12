use tracing::{info};

use serde::{Serialize, Deserialize};
use revolt_models::v0;

use axum::{
    extract::Json,
    extract::State
};
use chrono::Utc;
use crate::AppState;



#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "event_type")]
pub enum IngestEvent {
    MessageCreate { data: v0::Message },
}

pub async fn ingest(
    State(app): State<AppState>,
    Json(payload): Json<IngestEvent>,
) {
    info!("Ingesting {:?}", payload);
    match payload {
        IngestEvent::MessageCreate { data } => {
            let action_id = ulid::Ulid::from_string(&data.id)
                .expect("our ulids to be correct");
            let action_id = (action_id.0 >> 64) as u64;

            let payload = serde_json::json!({
                "send_time": Utc::now().to_rfc3339(),
                "data": {
                    "action_id": action_id,
                    "action_name": "create_post",
                    "data": {
                        "user_id": data.author,
                        "ip_address": "127.0.0.1",
                        "event_type": "create_post",
                        "post": {
                            "text": data.content.clone().unwrap_or_default()
                        }
                    }
                }
            }).to_string();

            let record = app.kafka.create_record()
                .payload(&payload)
                .key(&data.id);
            
            dbg!(app.kafka.enqueue(record).await.unwrap());
        }
    }
}