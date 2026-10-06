use crate::{error_response::RespErrorResponse, resp::RespValue};

pub fn validate_max_args(
    command_length: usize,
    command_length_limit: usize,
) -> Result<(), RespErrorResponse> {
    if command_length > command_length_limit {
        return Err(RespErrorResponse::InvalidRESPCommand(format!(
            "Command needs to be at most {} commands long, but got {}",
            command_length_limit, command_length
        )));
    }
    Ok(())
}

pub fn validate_min_args(
    command_length: usize,
    command_length_limit: usize,
) -> Result<(), RespErrorResponse> {
    if command_length < command_length_limit {
        return Err(RespErrorResponse::InvalidRESPCommand(format!(
            "Command needs to be at least {} commands long, but got {}",
            command_length_limit, command_length
        )));
    }
    Ok(())
}
