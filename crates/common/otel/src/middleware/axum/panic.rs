use std::any::Any;

use ::axum::response::{IntoResponse, Response};
use revolt_result::{Error, ErrorType};

pub fn panic_response(_panic: Box<dyn Any + Send + 'static>) -> Response {
    Error {
        error_type: ErrorType::InternalError,
        location: None,
    }
    .into_response()
}
