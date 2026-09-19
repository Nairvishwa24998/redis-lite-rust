use bytes::Bytes;

use crate::{command_handling_registry::{echo_handler, ping_handler}, constants::{ECHO_COMMAND, PING_COMMAND, SET_COMMAND, GET_COMMAND, EXISTS_COMMAND}, error_response::RespErrorResponse, resp::RespValue};

// Will take in the Deserialized RESP command and return appropriate response
pub fn command_handler(deserialized_commands_object: &RespValue) -> Result<RespValue, RespErrorResponse> {
    // Will always receive an array of BulkStrings according to official RESP documentation, so we should extract the array out of it
    if let RespValue::Array(deserialized_commands) = deserialized_commands_object {
        let deserialized_command = &deserialized_commands[0];
        match deserialized_command {
            // Every command in the array has to be a BulkString, so we can unwrap it here
            RespValue::BulkString(command) => {
                match command.as_ref(){
                    ECHO_COMMAND => Ok(echo_handler()),
                    PING_COMMAND => ping_handler(deserialized_commands),
                    SET_COMMAND => Ok(set_handler()),
                    GET_COMMAND => Ok(get_handler()),
                    EXISTS_COMMAND => Ok(exists_handler()), // Placeholder for EXISTS command
                    _ => Err(RespErrorResponse::InvalidRESPCommand),
                }

            }
            _ => Err(RespErrorResponse::InvalidRESPType),
        }
    }
    // We need to return array if command is not an array since RESP mandates this from the client side
    else {
        Err(RespErrorResponse::InvalidRESPType)
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


