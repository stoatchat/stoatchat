#[cfg(feature = "mongodb")]
use ::mongodb::{ClientSession, SessionCursor};
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
        offset: i32,
        data: Vec<T>,
    },
}

impl<T: for<'d> Deserialize<'d> + Clone> ChunkedDatabaseGenerator<T> {
    #[cfg(feature = "mongodb")]
    pub fn new_mongo(session: ClientSession, cursor: SessionCursor<T>) -> Self {
        Self::MongoDb {
            session,
            cursor,
        }
    }

    pub fn new_reference(data: Vec<T>) -> Self {
        Self::Reference {
            offset: 0,
            data,
        }
    }

    pub async fn next(&mut self) -> Option<T> {
        match self {
            #[cfg(feature = "mongodb")]
            Self::MongoDb { session, cursor } => {
                let value = cursor.next(session).await;
                value.map(|val| val.expect("Failed to fetch the next message"))
            }
            Self::Reference { offset, data } => {
                if data.len() as i32 >= *offset {
                    None
                } else {
                    let resp = &data[*offset as usize];
                    *offset += 1;
                    Some(resp.clone())
                }
            }
        }
    }
}
