use bytes::Bytes;

use crate::{
    error_response::RespErrorResponse,
    set_manager::{ExpiryKind, SetLifecycle},
    store::Store,
};

pub fn set_nx_handler(set_lifecycle: &mut SetLifecycle) -> Result<(), RespErrorResponse> {
    // xx is already set, so we cannot set nx
    if set_lifecycle.get_xx() {
        return Err(RespErrorResponse::InvalidRESPCommand(
            "Invalid SET command format".to_string(),
        ));
        // Can try proceed with setting nx
    }
    set_lifecycle.set_nx();
    Ok(())
}

pub fn set_xx_handler(set_lifecycle: &mut SetLifecycle) -> Result<(), RespErrorResponse> {
    // nx is already set, so we cannot set xx
    if set_lifecycle.get_nx() {
        return Err(RespErrorResponse::InvalidRESPCommand(
            "Invalid SET command format".to_string(),
        ));
        // Can try proceed with setting xx
    }
    set_lifecycle.set_xx();
    Ok(())
}

pub fn set_get_handler(set_lifecycle: &mut SetLifecycle) -> Result<(), RespErrorResponse> {
    set_lifecycle.set_get();
    Ok(())
}

pub fn set_expiry_handler(
    set_lifecycle: &mut SetLifecycle,
    expiry_kind: ExpiryKind,
    time_deadline: Option<&Bytes>,
    now: u64,
) -> Result<(), RespErrorResponse> {
    if let Some((existing_expiry_kind, _)) = set_lifecycle.get_expiry_info() {
        // Dont wanna allow multiple exp flags together
        if existing_expiry_kind != expiry_kind {
            return Err(RespErrorResponse::InvalidRESPCommand(
                "syntax error".to_string(),
            ));
        }
    }

    // Number or deadline following expiry commands not given then throw error
    // ok_or_else better here coz it does lazy evaluation as against eager evaluation
    let time_deadline_value = time_deadline
        .ok_or_else(|| RespErrorResponse::InvalidRESPCommand("syntax error".to_string()))?;

    let n: i64 = std::str::from_utf8(time_deadline_value)
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| {
            RespErrorResponse::InvalidRESPCommand(
                "value is not an integer or out of range".to_string(),
            )
        })?;

    if n <= 0 {
        return Err(RespErrorResponse::InvalidRESPCommand(
            "invalid expire time in 'set' command".to_string(),
        ));
    }

    let n = n as u64; // safe: n > 0
    let deadline = match expiry_kind {
        ExpiryKind::Ex => n.checked_mul(1000).and_then(|ms| ms.checked_add(now)),
        ExpiryKind::Px => n.checked_add(now),
        ExpiryKind::ExAt => n.checked_mul(1000),
        ExpiryKind::PxAt => Some(n),
    }
    .ok_or_else(|| {
        RespErrorResponse::InvalidRESPCommand("invalid expire time in 'set' command".to_string())
    })?;
    set_lifecycle.set_expiry_info(expiry_kind, deadline);
    Ok(())
}
