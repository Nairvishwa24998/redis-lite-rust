use bytes::Bytes;

use crate::{
    constants::PONG_COMMAND,
    error_response::RespErrorResponse,
    resp::RespValue,
    store::Store,
    utils::{validate_max_args, validate_min_args},
};

// We could have taken a Reference of a Vec of RespValue as well. But so we stick to a single stylistic choice we followed in the serializer
pub fn ping_handler(deserialized_commands: &[RespValue]) -> Result<RespValue, RespErrorResponse> {
    let command_len = deserialized_commands.len();
    validate_max_args(command_len, 2)?;
    // return just PONG as simple string
    if command_len == 1 {
        return Ok(RespValue::SimpleString(Bytes::from_static(PONG_COMMAND)));
    }
    // return PONG as bulk string if there is an argument
    let argument = &deserialized_commands[1];
    if let RespValue::BulkString(bytes) = argument {
        return Ok(RespValue::BulkString(bytes.clone()));
    }

    Err(RespErrorResponse::InvalidRESPCommand(
        "Invalid PING command format".to_string(),
    ))
}

pub fn echo_handler() -> RespValue {
    RespValue::SimpleString(Bytes::from_static(b""))
}

pub fn set_handler(
    deserialized_commands: &[RespValue],
    store: &mut Store,
) -> Result<RespValue, RespErrorResponse> {
    let command_len = deserialized_commands.len();
    validate_min_args(command_len, 3)?;
    // we can change as we support more commands
    validate_max_args(command_len, 3)?;
    let key_arg = &deserialized_commands[1];
    let val_arg = &deserialized_commands[2];
    if let (RespValue::BulkString(key), RespValue::BulkString(val)) = (key_arg, val_arg) {
        // Here we would call the store's set method to store the key-value pair
        store.set(key.clone(), val.clone());
        return Ok(RespValue::SimpleString(Bytes::from_static(b"OK")));
    }
    Err(RespErrorResponse::InvalidRESPCommand(
        "Invalid SET command format".to_string(),
    ))
}

pub fn get_handler(
    deserialized_commands: &[RespValue],
    store: &mut Store,
) -> Result<RespValue, RespErrorResponse> {
    let command_len = deserialized_commands.len();
    if command_len != 2 {
        return Err(RespErrorResponse::InvalidRESPCommand(
            "GET command requires exactly 1 key argument".to_string(),
        ));
    }
    let key_arg = &deserialized_commands[1];
    if let RespValue::BulkString(key) = key_arg {
        // Here we would call the store's get method to retrieve the value for the given key
        if let Some(value) = store.get(key) {
            return Ok(RespValue::BulkString(value.clone()));
        } else {
            return Ok(RespValue::Null);
        }
    }
    Err(RespErrorResponse::InvalidRESPCommand(
        "Invalid GET command format".to_string(),
    ))
}

pub fn exists_handler(
    deserialized_commands: &[RespValue],
    store: &mut Store,
) -> Result<RespValue, RespErrorResponse> {
    let command_len = deserialized_commands.len();
    validate_min_args(command_len, 2)?;
    let key_args = &deserialized_commands[1..];
    let mut count = 0;
    for key_arg in key_args {
        if let RespValue::BulkString(key) = key_arg {
            if store.get(key).is_some() {
                count += 1;
            }
        } else {
            return Err(RespErrorResponse::InvalidRESPCommand(
                "Invalid EXISTS command format".to_string(),
            ));
        }
    }
    Ok(RespValue::Integer(count))
}
