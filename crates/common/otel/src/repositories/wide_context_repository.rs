use std::sync::{Arc, Mutex};

use opentelemetry::{Key, logs::AnyValue};

#[derive(Clone, Default)]
pub struct WideContextRepository {
    attributes: Arc<Mutex<Vec<(Key, AnyValue)>>>,
}

impl WideContextRepository {
    pub fn add(&self, key: impl Into<Key>, value: impl Into<AnyValue>) {
        self.attributes
            .lock()
            .unwrap()
            .push((key.into(), value.into()));
    }

    pub(crate) fn take(&self) -> Vec<(Key, AnyValue)> {
        std::mem::take(&mut *self.attributes.lock().unwrap())
    }
}
