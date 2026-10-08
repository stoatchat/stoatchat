use std::sync::{Arc, Mutex};

use opentelemetry::{Key, logs::AnyValue};

#[derive(Clone, Default)]
pub struct WideContextRepository {
    attributes: Arc<Mutex<Vec<(Key, AnyValue)>>>,
}

impl WideContextRepository {
    pub(crate) fn take(&self) -> Vec<(Key, AnyValue)> {
        std::mem::take(&mut *self.attributes.lock().unwrap())
    }
}
