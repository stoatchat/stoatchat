use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
/// Coalescion service error.
pub enum Error {
    /// Failed to receive the actions return from the channel for unknown reason
    RecvError,
    /// Reached the `max_concurrent` amount of actions running at once and could not queue the action
    MaxConcurrent,
    /// Reached the `max_queue` amount of actions in the queue
    MaxQueue,
    /// Failed to downcast the type to the current type being returned, this will be most likely an ID collision
    DowncastError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::RecvError => write!(f, "Unable to receive data from the channel"),
            Error::MaxConcurrent => write!(f, "Max number of tasks running at once"),
            Error::MaxQueue => write!(f, "Max number of tasks in queue"),
            Error::DowncastError => write!(f, "Failed to downcast type, possible key collision with different types")
        }
    }
}

impl std::error::Error for Error {}
