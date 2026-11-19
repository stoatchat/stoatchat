#[cfg(feature = "amqp")]
mod amqp_log_impl;
mod otel_log_impl;

pub(crate) use otel_log_impl::WIDE_LOGGER;
