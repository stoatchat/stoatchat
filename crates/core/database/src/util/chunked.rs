#[cfg(feature = "mongodb")]
use ::mongodb::{ClientSession, SessionCursor};
use revolt_result::{Result, ToRevoltError};
use serde::Deserialize;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ChunkedDatabaseGenerator<T> {
    #[cfg(feature = "mongodb")]
    MongoDb {
        session: ClientSession,
        cursor: SessionCursor<T>,
    },

    Reference {
        offset: usize,
        data: Vec<T>,
    },
}

impl<T: for<'d> Deserialize<'d> + Clone> ChunkedDatabaseGenerator<T> {
    #[cfg(feature = "mongodb")]
    pub fn new_mongo(session: ClientSession, cursor: SessionCursor<T>) -> Self {
        Self::MongoDb { session, cursor }
    }

    pub fn new_reference(data: Vec<T>) -> Self {
        Self::Reference { offset: 0, data }
    }

    pub async fn next(&mut self) -> Result<Option<T>> {
        match self {
            #[cfg(feature = "mongodb")]
            Self::MongoDb { session, cursor } => {
                cursor.next(session).await.transpose().to_internal_error()
            }
            Self::Reference { offset, data } => {
                if let Some(value) = data.get(*offset) {
                    *offset += 1;
                    Ok(Some(value.clone()))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
