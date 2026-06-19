#[allow(clippy::module_inception)]
pub mod amqp;

pub use amqp::{AMQP, get_amqp};