use std::{any::Any, collections::HashMap, fmt::Debug, future::Future, hash::Hash, sync::Arc};

use tokio::sync::{
    watch::{channel as watch_channel, Receiver},
    RwLock,
};

#[cfg(feature = "cache")]
use lru::LruCache;

#[cfg(feature = "queue")]
use indexmap::IndexMap;

use crate::{CoalescionServiceConfig, Error};

#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
/// # Coalescion service
///
/// See module description for example usage.
pub struct CoalescionService<Id: Hash + Clone + Eq> {
    config: Arc<CoalescionServiceConfig>,
    watchers: Arc<RwLock<HashMap<Id, Receiver<Option<Result<Arc<dyn Any + Send + Sync>, Error>>>>>>,
    #[cfg(feature = "queue")]
    queue: Arc<RwLock<IndexMap<Id, Receiver<Option<Result<Arc<dyn Any + Send + Sync>, Error>>>>>>,
    #[cfg(feature = "cache")]
    cache: Option<Arc<tokio::sync::Mutex<LruCache<Id, Arc<dyn Any + Send + Sync>>>>>,
}

impl<Id: Hash + Clone + Eq> CoalescionService<Id> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_config(config: CoalescionServiceConfig) -> Self {
        Self {
            config: Arc::new(config),
            watchers: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "queue")]
            queue: Arc::new(RwLock::new(IndexMap::new())),
            #[cfg(feature = "cache")]
            cache: None,
        }
    }

    #[cfg(feature = "cache")]
    pub fn from_cache(
        config: CoalescionServiceConfig,
        cache: LruCache<Id, Arc<dyn Any + Send + Sync>>,
    ) -> Self {
        Self {
            cache: Some(Arc::new(Mutex::new(cache))),
            ..Self::from_config(config)
        }
    }

    async fn wait_for<Value: Any + Send + Sync>(
        &self,
        mut receiver: Receiver<Option<Result<Arc<dyn Any + Send + Sync>, Error>>>,
    ) -> Result<Arc<Value>, Error> {
        receiver
            .wait_for(|v| v.is_some())
            .await
            .map_err(|_| Error::RecvError)
            .and_then(|r| r.clone().unwrap())
            .and_then(|arc| Arc::downcast(arc).map_err(|_| Error::DowncastError))
    }

    async fn insert_and_execute<
        Value: Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Value>,
    >(
        &self,
        id: Id,
        func: F,
    ) -> Result<Arc<Value>, Error> {
        let (send, recv) = watch_channel(None);

        self.watchers.write().await.insert(id.clone(), recv);

        let value = Ok(Arc::new(func().await));

        send.send_modify(|opt| {
            opt.replace(value.clone().map(|v| v as Arc<dyn Any + Send + Sync>));
        });

        #[cfg(feature = "cache")]
        if let Some(cache) = self.cache.as_ref() {
            if let Ok(value) = &value {
                cache.lock().await.push(id.clone(), value.clone());
            }
        };

        self.watchers.write().await.remove(&id);

        value
    }

    /// Coalesces an function, the actual function may not run if one with the same id is already running,
    /// queued to be ran, or cached, the id should be globally unique for this specific action.
    pub async fn execute<
        Value: Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Value>,
    >(
        &self,
        id: Id,
        func: F,
    ) -> Result<Arc<Value>, Error> {
        #[cfg(feature = "cache")]
        if let Some(cache) = self.cache.as_ref() {
            if let Some(value) = cache.lock().await.get(&id) {
                return Arc::downcast::<Value>(value.clone()).map_err(|_| Error::DowncastError);
            }
        };

        let (receiver, length) = {
            let watchers = self.watchers.read().await;
            let length = watchers.len();

            (watchers.get(&id).cloned(), length)
        };

        if let Some(receiver) = receiver {
            self.wait_for(receiver).await
        } else {
            match self.config.max_concurrent {
                Some(max_concurrent) if length >= max_concurrent => {
                    #[cfg(feature = "queue")]
                    if self.config.queue_requests {
                        let (receiver, length) = {
                            let queue = self.queue.read().await;

                            (queue.get(&id).cloned(), queue.len())
                        };

                        if let Some(receiver) = receiver {
                            return self.wait_for(receiver).await;
                        } else {
                            if self
                                .config
                                .max_queue
                                .is_some_and(|max_queue| max_queue >= length)
                            {
                                return Err(Error::MaxQueue);
                            };

                            let (send, recv) = watch_channel(None);

                            self.queue.write().await.insert(id.clone(), recv);

                            loop {
                                let length = self.watchers.read().await.len();

                                if length < max_concurrent {
                                    let first_key = {
                                        let queue = self.queue.read().await;
                                        queue.first().map(|v| v.0).cloned()
                                    };

                                    if first_key == Some(id.clone()) {
                                        self.queue.write().await.shift_remove(&id);

                                        let response = self.insert_and_execute(id, func).await;

                                        send.send_modify(|opt| {
                                            opt.replace(
                                                response
                                                    .clone()
                                                    .map(|v| v as Arc<dyn Any + Send + Sync>),
                                            );
                                        });

                                        return response;
                                    }
                                }
                            }
                        }
                    } else {
                        Err(Error::MaxConcurrent)
                    }

                    #[cfg(not(feature = "queue"))]
                    Err(Error::MaxConcurrent)
                }
                _ => self.insert_and_execute(id, func).await,
            }
        }
    }

    /// Fetches the amount of currently running tasks
    pub async fn current_task_count(&self) -> usize {
        self.watchers.read().await.len()
    }

    #[cfg(feature = "queue")]
    /// Fetches the current length of the queue
    pub async fn current_queue_len(&self) -> usize {
        self.queue.read().await.len()
    }
}

impl<Id: Hash + Clone + Eq> Default for CoalescionService<Id> {
    fn default() -> Self {
        Self::from_config(CoalescionServiceConfig::default())
    }
}
