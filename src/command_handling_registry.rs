use bytes::Bytes;

use crate::{constants::PONG_COMMAND, error_response::RespErrorResponse, resp::RespValue};

// We could have taken a Reference of a Vec of RespValue as well. But so we stick to a single stylistic choice we followed in the serializer
pub fn ping_handler(deserialized_commands: &[RespValue]) -> Result<RespValue, RespErrorResponse> {
    let command_len = deserialized_commands.len();
    if command_len > 2 {
        return Err(RespErrorResponse::InvalidCommandArguments);
    }
    // return just PONG as simple string
    if command_len == 1 {
            return Ok(RespValue::SimpleString(Bytes::from_static(PONG_COMMAND)))
    }
    // return PONG as bulk string if there is an argument
    let argument = &deserialized_commands[1];
    if let RespValue::BulkString(bytes) = argument {
        return Ok(RespValue::BulkString(bytes.clone()));
    }

    Err(RespErrorResponse::InvalidCommandArguments)
}

pub fn echo_handler() -> RespValue {
    RespValue::SimpleString(Bytes::from_static(b""))
}
