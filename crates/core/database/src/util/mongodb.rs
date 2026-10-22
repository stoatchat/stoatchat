/// Whether a MongoDB error represents a duplicate key (unique index) violation.
///
/// Covers both bulk inserts (`InsertMany`) and single writes, matching server
/// error code 11000.
pub fn is_duplicate_key(err: &mongodb::error::Error) -> bool {
    use mongodb::error::{ErrorKind, WriteFailure};
    match &*err.kind {
        ErrorKind::InsertMany(e) => e
            .write_errors
            .as_ref()
            .map_or(false, |errs| errs.iter().all(|we| we.code == 11000)),
        ErrorKind::Write(WriteFailure::WriteError(we)) => we.code == 11000,
        _ => false,
    }
}
