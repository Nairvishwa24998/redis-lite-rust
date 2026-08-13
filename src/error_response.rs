use std::io::Error;


#[derive(Debug, PartialEq)]
pub enum RespErrorResponse {
    IncorrectTypeError,
    UtfError,
    // Buffer doesn't yet hold a complete value — wait for more bytes, don't drop the connection.
    // Replaces the old EmptyInput/MissingCRLF/GenericError, which all fired on exactly this
    // ambiguous condition (see progress.md, 2026-08-09 "OPEN QUESTION" entry).
    Incomplete,
    AttributeMismatchError,
    InvalidRESPType,
    BulkStringTooLarge,
}

pub enum CustomErrorType {
    IncorrectPortNumber,
}
pub struct CustomErrorResponse {
    error_type: CustomErrorType,
    message: String,
}

// Needed to map the error from default Error to our CustomErrorResponse
impl From<Error> for CustomErrorResponse {
    fn from(err: Error) -> Self {
        CustomErrorResponse {
            error_type: CustomErrorType::IncorrectPortNumber,
            message: err.to_string(),
        }
    }
}
