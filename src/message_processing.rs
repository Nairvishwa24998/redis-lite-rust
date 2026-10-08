use bytes::Bytes;

use crate::{
    command_handling_registry::{
        echo_handler, exists_handler, get_handler, ping_handler, set_handler,
    },
    constants::{ECHO_COMMAND, EXISTS_COMMAND, GET_COMMAND, PING_COMMAND, SET_COMMAND},
    error_response::RespErrorResponse,
    resp::RespValue,
    store::Store,
};

// Will take in the Deserialized RESP command and return appropriate response
pub fn command_handler(
    deserialized_commands_object: &RespValue,
    store: &mut Store,
) -> Result<RespValue, RespErrorResponse> {
    // Will always receive an array of BulkStrings according to official RESP documentation, so we should extract the array out of it
    if let RespValue::Array(deserialized_commands) = deserialized_commands_object {
        // deserialized_commands is already a reference so no needfor extra reference
        let deserialized_command = deserialized_commands.first().ok_or_else(|| {
            RespErrorResponse::InvalidRESPCommand("Empty command array".to_string())
        })?;
        match deserialized_command {
            // Every command in the array has to be a BulkString, so we can unwrap it here
            RespValue::BulkString(command) => {
                match command.as_ref() {
                    ECHO_COMMAND => Ok(echo_handler()),
                    PING_COMMAND => ping_handler(deserialized_commands),
                    SET_COMMAND => set_handler(deserialized_commands, store),
                    GET_COMMAND => get_handler(deserialized_commands, store),
                    EXISTS_COMMAND => exists_handler(deserialized_commands, store), // Placeholder for EXISTS command
                    _ => Err(RespErrorResponse::InvalidRESPCommand(format!(
                        "Unknown command: {}",
                        // command is already a reference so no needfor extra reference
                        String::from_utf8_lossy(command)
                    ))),
                }
            }
            _ => Err(RespErrorResponse::InvalidRESPCommand(
                "Command must be a BulkString".to_string(),
            )),
        }
    }
    // We need to return array if command is not an array since RESP mandates this from the client side
    else {
        Err(RespErrorResponse::InvalidRESPCommand(
            "Expected an array of commands".to_string(),
        ))
    }
}

fn del_handler() -> RespValue {
    RespValue::BulkString(Bytes::from_static(b"Hello, World!"))
}

fn incr_handler() -> RespValue {
    RespValue::BulkString(Bytes::from_static(b"Hello, World!"))
}

fn decr_handler() -> RespValue {
    RespValue::BulkString(Bytes::from_static(b"Hello, World!"))
}

fn l_push_handler() -> RespValue {
    RespValue::BulkString(Bytes::from_static(b"Hello, World!"))
}

pub fn map_error_to_resp_object(error_response: &RespErrorResponse) -> RespValue {
    match error_response {
        RespErrorResponse::InvalidRESPCommand(error_message) => RespValue::Error(Bytes::from(
            format!("ERR {}", error_message).replace(['\r', '\n'], " "),
        )),
        // Deserializer errors. Connection gets closed after this reply is flushed
        RespErrorResponse::IncorrectTypeError | RespErrorResponse::UtfError => RespValue::Error(
            Bytes::from_static(b"ERR Protocol error: invalid length or integer"),
        ),
        RespErrorResponse::BulkStringTooLarge => {
            RespValue::Error(Bytes::from_static(b"ERR Protocol error: invalid bulk length"))
        }
        RespErrorResponse::AttributeMismatchError => RespValue::Error(Bytes::from_static(
            b"ERR Protocol error: bulk length does not match data",
        )),
        RespErrorResponse::InvalidRESPType => {
            RespValue::Error(Bytes::from_static(b"ERR Protocol error: invalid type byte"))
        }
        // Never reaches here, Incomplete means wait for more bytes
        RespErrorResponse::Incomplete => RespValue::Error(Bytes::from_static(b"ERR Unknown error")),
    }
}
