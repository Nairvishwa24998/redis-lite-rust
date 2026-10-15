use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;

use crate::{
    constants::PONG_COMMAND,
    error_response::RespErrorResponse,
    resp::RespValue,
    set_command_handling_registry::{
        set_expiry_handler, set_get_handler, set_nx_handler, set_xx_handler,
    },
    set_manager::{ExpiryKind, SetLifecycle},
    store::Store,
    utils::{validate_max_args, validate_min_args},
};

// We could have taken a Reference of a Vec of RespValue as well. But so we stick to a single stylistic choice we followed in the serializer
pub fn ping_handler(validated_command_bytes: &[Bytes]) -> Result<RespValue, RespErrorResponse> {
    let command_len = validated_command_bytes.len();
    validate_max_args(command_len, 2)?;
    // return just PONG as simple string
    if command_len == 1 {
        return Ok(RespValue::SimpleString(Bytes::from_static(PONG_COMMAND)));
    }
    // return PONG as bulk string if there is an argument
    let argument = &validated_command_bytes[1];

    return Ok(RespValue::BulkString(argument.clone()));
}

pub fn echo_handler(validated_command_bytes: &[Bytes]) -> Result<RespValue, RespErrorResponse> {
    let command_len = validated_command_bytes.len();
    if command_len == 2 {
        let argument = &validated_command_bytes[1];
        return Ok(RespValue::BulkString(argument.clone()));
    } else {
        Err(RespErrorResponse::InvalidRESPCommand(
            "Invalid ECHO command format".to_string(),
        ))
    }
}

pub fn set_handler(
    validated_command_bytes: &[Bytes],
    store: &mut Store,
) -> Result<RespValue, RespErrorResponse> {
    let command_len = validated_command_bytes.len();
    validate_min_args(command_len, 3)?;
    // we can change as we support more commands
    validate_max_args(command_len, 7)?;

    let key_arg = &validated_command_bytes[1];
    let val_arg = &validated_command_bytes[2];

    // Basic case
    if command_len == 3 {
        store.set(key_arg.clone(), val_arg.clone());
        return Ok(RespValue::SimpleString(Bytes::from_static(b"OK")));
    }

    let mut set_lifecycle = SetLifecycle::new();

    let mut index = 0;
    let remaining_args = &validated_command_bytes[3..];

    // we cast to u64 coz default u128 for sys time is just for extra safety. We dont need that extra space for
    // most practical purposes
    // let now: u64 = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)
    //     .ok()
    //     .and_then(|duration| u64::try_from(duration.as_millis()).ok())
    //     .ok_or_else(|| RespErrorResponse::InvalidRESPCommand("system clock error".to_string()))?;

    // Above is also right. This is just a simpleer way to do the same
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RespErrorResponse::InvalidRESPCommand("system clock error".to_string()))?
        .as_millis() as u64;

    while index < remaining_args.len() {
        let arg = &remaining_args[index];
        // So we can allow case insensitivity
        match arg.to_ascii_uppercase().as_slice() {
            b"NX" => {
                let _ = set_nx_handler(&mut set_lifecycle)?;
                index += 1;
            }
            b"XX" => {
                let _ = set_xx_handler(&mut set_lifecycle)?;
                index += 1;
            }
            b"GET" => {
                let _ = set_get_handler(&mut set_lifecycle)?;
                index += 1
            }
            b"EX" => {
                set_expiry_handler(
                    &mut set_lifecycle,
                    ExpiryKind::Ex,
                    remaining_args.get(index + 1),
                    now,
                )?;

                index += 2;
            }
            b"PX" => {
                set_expiry_handler(
                    &mut set_lifecycle,
                    ExpiryKind::Px,
                    remaining_args.get(index + 1),
                    now,
                )?;
                index += 2;
            }
            b"EXAT" => {
                set_expiry_handler(
                    &mut set_lifecycle,
                    ExpiryKind::ExAt,
                    remaining_args.get(index + 1),
                    now,
                )?;
                index += 2;
            }
            b"PXAT" => {
                set_expiry_handler(
                    &mut set_lifecycle,
                    ExpiryKind::PxAt,
                    remaining_args.get(index + 1),
                    now,
                )?;
                index += 2;
            }

            _ => {
                return Err(RespErrorResponse::InvalidRESPCommand(
                    "Invalid SET command format".to_string(),
                ));
            }
        }
    }

    Err(RespErrorResponse::InvalidRESPCommand(
        "Invalid SET command format".to_string(),
    ))
}

pub fn get_handler(
    validated_command_bytes: &[Bytes],
    store: &mut Store,
) -> Result<RespValue, RespErrorResponse> {
    let command_len = validated_command_bytes.len();
    if command_len != 2 {
        return Err(RespErrorResponse::InvalidRESPCommand(
            "GET command requires exactly 1 key argument".to_string(),
        ));
    }
    let key_arg = &validated_command_bytes[1];

    // Here we would call the store's get method to retrieve the value for the given key
    if let Some(value) = store.get(key_arg) {
        return Ok(RespValue::BulkString(value.clone()));
    } else {
        return Ok(RespValue::Null);
    }
}

pub fn exists_handler(
    validated_command_bytes: &[Bytes],
    store: &mut Store,
) -> Result<RespValue, RespErrorResponse> {
    let command_len = validated_command_bytes.len();
    validate_min_args(command_len, 2)?;
    let key_args = &validated_command_bytes[1..];
    let mut count = 0;
    for key_arg in key_args {
        if store.get(key_arg).is_some() {
            count += 1;
        }
    }
    Ok(RespValue::Integer(count))
}
