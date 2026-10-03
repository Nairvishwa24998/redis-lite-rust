use std::io::Error;

#[derive(Debug, PartialEq)]
pub enum RespErrorResponse {
    // Buffer doesn't yet hold a complete value — wait for more bytes, don't drop the connection.
    Incomplete,
    // leads to client disconnect
    IncorrectTypeError,
    UtfError,
    AttributeMismatchError,
    InvalidRESPType,
    BulkStringTooLarge,
    // Doesn't disconnect and used for specific errors
    InvalidRESPCommand(String)
}

#[derive(Debug, PartialEq)]
pub enum ServerErrorType {
    IncorrectPortNumber,
}

#[derive(Debug, PartialEq)]
pub struct ServerError {
    error_type: ServerErrorType,
    message: String,
}

// Needed to map the error from default Error to our ServerError
impl From<Error> for ServerError {
    fn from(err: Error) -> Self {
        ServerError {
            error_type: ServerErrorType::IncorrectPortNumber,
            message: err.to_string(),
        }
    }
}
