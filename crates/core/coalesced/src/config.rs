#[derive(Clone, PartialEq, Eq, Debug)]
/// Config values for [`CoalescionService`].
pub struct CoalescionServiceConfig {
    /// How many tasks are running at once
    pub max_concurrent: Option<usize>,
    /// Whether to queue tasks once `max_concurrent` is reached
    #[cfg(feature = "queue")]
    pub queue_requests: bool,
    /// Max amount of tasks in the buffer queue
    #[cfg(feature = "queue")]
    pub max_queue: Option<usize>,
}

impl Default for CoalescionServiceConfig {
    fn default() -> Self {
        Self {
            max_concurrent: Some(100),
            #[cfg(feature = "queue")]
            queue_requests: true,
            #[cfg(feature = "queue")]
            max_queue: Some(100)
        }
    }
}
