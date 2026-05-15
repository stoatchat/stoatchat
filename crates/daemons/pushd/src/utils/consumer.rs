use std::{
    future::{ready, Future},
    pin::Pin,
    sync::Arc,
};

use async_trait::async_trait;
use lapin::{
    message::{Delivery, DeliveryResult},
    options::BasicPublishOptions,
    Channel, Connection, ConsumerDelegate,
};
use log::{debug, warn};
use revolt_database::Database;
use revolt_result::Result;

#[async_trait]
pub trait Consumer: Clone + Send + Sync + 'static {
    fn create(
        db: Database,
        authifier_db: authifier::Database,
        connection: Arc<Connection>,
        channel: Arc<Channel>,
    ) -> Self;
    fn channel(&self) -> &Arc<Channel>;
    async fn consume(&self, delivery: Delivery) -> Result<()>;

    async fn publish_message(&self, payload: Vec<u8>, args: BasicPublishOptions) {
        todo!()
    }
}

pub struct Delegate<C: Consumer>(pub C);

impl<C: Consumer> ConsumerDelegate for Delegate<C> {
    fn on_new_delivery(
        &self,
        delivery: DeliveryResult,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        match delivery {
            Ok(Some(delivery)) => {
                let consumer = self.0.clone();

                Box::pin(async move {
                    if let Err(e) = consumer.consume(delivery).await {
                        log::error!("{e:?}");
                    };
                })
            }
            Ok(None) => Box::pin(ready(())),
            Err(e) => Box::pin(async move { log::error!("Received bad delivery: {e:?}") }),
        }
    }
}
