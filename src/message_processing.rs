use bytes::Bytes;

use crate::{
    command_handling_registry::{echo_handler, ping_handler},
    constants::{ECHO_COMMAND, EXISTS_COMMAND, GET_COMMAND, PING_COMMAND, SET_COMMAND},
    error_response::RespErrorResponse,
    resp::RespValue,
};

// Will take in the Deserialized RESP command and return appropriate response
pub fn command_handler(
    deserialized_commands_object: &RespValue,
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
                    SET_COMMAND => Ok(set_handler()),
                    GET_COMMAND => Ok(get_handler()),
                    EXISTS_COMMAND => Ok(exists_handler()), // Placeholder for EXISTS command
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

fn set_handler() -> RespValue {
    RespValue::SimpleString(Bytes::from_static(b"OK"))
}

fn get_handler() -> RespValue {
    RespValue::BulkString(Bytes::from_static(b"Hello, World!"))
}

fn exists_handler() -> RespValue {
    RespValue::BulkString(Bytes::from_static(b"Hello, World!"))
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
    if let RespErrorResponse::InvalidRESPCommand(error_message) = error_response {
        RespValue::Error(Bytes::from(
            format!("ERR {}", error_message).replace(['\r', '\n'], " "),
        ))
    } else {
        RespValue::Error(Bytes::from_static(b"ERR Unknown error"))
    }
}
