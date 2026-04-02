use revolt_models::v0;

/// Wrapper for the Osprey client, managed by Rocket
pub struct OspreyClient(reqwest::Client, String);

impl OspreyClient {
    pub fn connect(host: String) -> Self {
        Self(reqwest::Client::new(), host)
    }

    pub async fn publish_message(&self, m: &v0::Message) {
        let json = serde_json::json!({
            "event_type": "MessageCreate",
            "data": m
        });
        let res = self.0.post(format!("{}/ingest", self.1))
            .json(&json)
            .send()
            .await;
        dbg!(res);
    }
}
