use crate::constants::{
    ARRAY_FIRST_BYTE, BULK_STRING_FIRST_BYTE, CRLF_BYTES_OFFSET, ERROR_FIRST_BYTE,
    INTEGER_FIRST_BYTE, MAX_BULK_STRING_LEN, NULL_ARRAY, NULL_BULK_STRING,
    SIMPLE_STRING_FIRST_BYTE,
};
use crate::error_response::RespErrorResponse;
use crate::resp::RespValue;
use bytes::Bytes;

pub fn deserializer(byte_resp: &[u8]) -> Result<(RespValue, usize), RespErrorResponse> {
    // We need to pass in the cursor for possible recursive calls where we might only be checking a slice
    // preliminary_validate_resp(byte_resp, 0)?;
    //   We initialize cursor to be 0
    parse_serialized_input(byte_resp, 0)
}

//Method is not needed
// // Since its rust we wont have a not type string issue
// fn preliminary_validate_resp(byte_resp: &[u8], cursor: usize) -> Result<(), RespErrorResponse> {
//     if byte_resp.is_empty() {
//         // eprintln!("Input cannot be null or empty!");
//         return Err(RespErrorResponse::EmptyInput);
//     } else if byte_resp.len() < RESP_MIN_LEGAL_LENGTH {
//         // eprintln!("Input is too short to be a valid RESP!");
//         return Err(RespErrorResponse::GenericError);
//     }
//     // Happy Path
//     Ok(())
// }

// Returns resultant value and cursor position in a tuple
fn parse_serialized_input(
    byte_resp: &[u8],
    cursor: usize,
) -> Result<(RespValue, usize), RespErrorResponse> {
    // We don't need to check for empty input here since we already did that in the preliminary validation step. We also don't need to check for length since we are checking for NULL values which are of length 5 and 6 respectively, so if the length is less than that, it will be caught in the preliminary validation step.
    // Preliminary check to see if its a RESP rep of NULL, if yes, we return and move cursor forward
    if byte_resp.get(cursor..cursor + NULL_ARRAY.len()) == Some(&NULL_ARRAY[..]) {
        let new_cursor = cursor + NULL_ARRAY.len();
        return Ok((RespValue::Null, new_cursor));
    } else if byte_resp.get(cursor..cursor + NULL_BULK_STRING.len()) == Some(&NULL_BULK_STRING[..])
    {
        let new_cursor: usize = cursor + NULL_BULK_STRING.len();
        return Ok((RespValue::Null, new_cursor));
    }

    let first_byte: u8 = match byte_resp.get(cursor) {
        Some(byte) => *byte,
        None => return Err(RespErrorResponse::Incomplete),
    };

    // We push one forward to avoid reading the sign indicator at rhe beginning
    let new_cursor = cursor + 1;
    match first_byte {
        BULK_STRING_FIRST_BYTE => parse_serialized_bulk_string(byte_resp, new_cursor),
        SIMPLE_STRING_FIRST_BYTE => {
            parse_serialize_error_or_simple_string(byte_resp, new_cursor, true)
        }
        INTEGER_FIRST_BYTE => parse_serialized_int(byte_resp, new_cursor),
        ARRAY_FIRST_BYTE => parse_serialize_array(byte_resp, new_cursor),
        ERROR_FIRST_BYTE => parse_serialize_error_or_simple_string(byte_resp, new_cursor, false),
        // For any other type which is not dealt by RESP
        _ => Err(RespErrorResponse::InvalidRESPType),
    }
}
fn parse_serialized_int(
    byte_resp: &[u8],
    cursor: usize,
) -> Result<(RespValue, usize), RespErrorResponse> {
    let crlf_position: usize = validate_and_fetch_crlf_pos(byte_resp, cursor)?;
    let stringified_integer_value: &str = std::str::from_utf8(&byte_resp[cursor..crlf_position])
        .map_err(|_| RespErrorResponse::UtfError)?;
    let integer_value: i64 = stringified_integer_value
        .parse()
        .map_err(|_| RespErrorResponse::IncorrectTypeError)?;
    let new_cursor: usize = crlf_position + CRLF_BYTES_OFFSET;
    Ok((RespValue::Integer(integer_value), new_cursor))
}

fn parse_serialized_bulk_string(
    byte_resp: &[u8],
    cursor: usize,
) -> Result<(RespValue, usize), RespErrorResponse> {
    // Find where the length digits end. Easier way to write instead of using match expressions
    let crlf_position = validate_and_fetch_crlf_pos(byte_resp, cursor)?;

    // Turn those digit bytes into an actual number
    let count_str: &str = std::str::from_utf8(&byte_resp[cursor..crlf_position])
        .map_err(|_| RespErrorResponse::UtfError)?;
    // Converting potential number to integer
    let count: usize = count_str
        .parse()
        .map_err(|_| RespErrorResponse::IncorrectTypeError)?;
    // Reject absurd lengths before any arithmetic touches `count` — keeps the additions
    // below nowhere near usize overflow, regardless of what a client claims.
    if count > MAX_BULK_STRING_LEN {
        return Err(RespErrorResponse::BulkStringTooLarge);
    }
    // 3. skip the CRLF after the length, slice out `count` bytes of data
    let data_start: usize = crlf_position + CRLF_BYTES_OFFSET;

    let expected_data_end: usize = data_start + count;
    // No potential space for CRLF so, show error
    if expected_data_end + 1 >= byte_resp.len() {
        return Err(RespErrorResponse::Incomplete);
    }
    // Given count is too high, we finished the lnegth of the bulkstring but still didnt reach the end crlf
    if byte_resp[expected_data_end] != b'\r' || byte_resp[expected_data_end + 1] != b'\n' {
        return Err(RespErrorResponse::AttributeMismatchError);
    }
    // Reached here means everything has been adhered to
    let data_end = expected_data_end;

    let data: &[u8] = &byte_resp[data_start..data_end];

    // 4. skip the CRLF after the data — that's where the next value starts
    let new_cursor = data_end + CRLF_BYTES_OFFSET;

    Ok((RespValue::BulkString(map_to_bytes(data)), new_cursor))
}

// We use a simple flag, true means simple string, false means error
fn parse_serialize_error_or_simple_string(
    byte_resp: &[u8],
    cursor: usize,
    string_error_flag: bool,
) -> Result<(RespValue, usize), RespErrorResponse> {
    let crlf_position: usize = validate_and_fetch_crlf_pos(byte_resp, cursor)?;
    let data_value: &[u8] = &byte_resp[cursor..crlf_position];
    let new_cursor: usize = crlf_position + CRLF_BYTES_OFFSET;
    if string_error_flag {
        return Ok((
            RespValue::SimpleString(map_to_bytes(data_value)),
            new_cursor,
        ));
    } else {
        return Ok((RespValue::Error(map_to_bytes(data_value)), new_cursor));
    }
}

fn parse_serialize_array(
    byte_resp: &[u8],
    cursor: usize,
) -> Result<(RespValue, usize), RespErrorResponse> {
    let mut array_elements: Vec<RespValue> = Vec::new();
    let crlf_position = validate_and_fetch_crlf_pos(byte_resp, cursor)?;
    // Turn those digit bytes into an actual number
    let count_str: &str = std::str::from_utf8(&byte_resp[cursor..crlf_position])
        .map_err(|_| RespErrorResponse::UtfError)?;
    // Converting potential number to integer
    let count: usize = count_str
        .parse()
        .map_err(|_| RespErrorResponse::IncorrectTypeError)?;
    // to push past the initial number of element bit
    let mut new_cursor: usize = crlf_position + CRLF_BYTES_OFFSET;
    // Can put _ since we don't need the index, we just need to loop count times
    for _ in 0..count {
        let (temp_result, temp_cursor) = parse_serialized_input(byte_resp, new_cursor)?;
        array_elements.push(temp_result);
        new_cursor = temp_cursor;
    }
    return Ok((RespValue::Array(array_elements), new_cursor));
}

/// Finds the index of the `\r` in the first CRLF at or after `cursor`.
/// Used to locate the end of a length/count prefix (e.g. the `5` in `$5\r\nhello\r\n`).
/// Returns `Err(Incomplete)` if none is found — the buffer may just not have the terminator yet.
fn validate_and_fetch_crlf_pos(
    byte_resp: &[u8],
    cursor: usize,
) -> Result<usize, RespErrorResponse> {
    let max_len = byte_resp.len();
    for i in cursor..max_len {
        // found CRLF. Only check till the second last element to prevent out of bounds issues
        if i < max_len - 1 && byte_resp[i] == b'\r' && byte_resp[i + 1] == b'\n' {
            return Ok(i);
        }
    }
    return Err(RespErrorResponse::Incomplete);
}

// We take the incoming network bytes and convert it to our Bytes format
fn map_to_bytes(data: &[u8]) -> Bytes {
    Bytes::copy_from_slice(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_null_bulk_string() {
        let (value, cursor) = deserializer(b"$-1\r\n").unwrap();
        assert_eq!(value, RespValue::Null);
        assert_eq!(cursor, 5);
    }

    #[test]
    fn parses_null_array() {
        let (value, cursor) = deserializer(b"*-1\r\n").unwrap();
        assert_eq!(value, RespValue::Null);
        assert_eq!(cursor, 5);
    }

    #[test]
    fn parses_simple_string() {
        let (value, cursor) = deserializer(b"+OK\r\n").unwrap();
        assert_eq!(value, RespValue::SimpleString(Bytes::from_static(b"OK")));
        assert_eq!(cursor, 5);
    }

    #[test]
    fn parses_error() {
        let (value, cursor) = deserializer(b"-oh no\r\n").unwrap();
        assert_eq!(value, RespValue::Error(Bytes::from_static(b"oh no")));
        assert_eq!(cursor, 8);
    }

    #[test]
    fn parses_positive_integer() {
        let (value, cursor) = deserializer(b":123\r\n").unwrap();
        assert_eq!(value, RespValue::Integer(123));
        assert_eq!(cursor, 6);
    }

    #[test]
    fn parses_negative_integer() {
        let (value, cursor) = deserializer(b":-5\r\n").unwrap();
        assert_eq!(value, RespValue::Integer(-5));
        assert_eq!(cursor, 5);
    }

    #[test]
    fn parses_bulk_string() {
        let (value, cursor) = deserializer(b"$5\r\nhello\r\n").unwrap();
        assert_eq!(value, RespValue::BulkString(Bytes::from_static(b"hello")));
        assert_eq!(cursor, 11);
    }

    #[test]
    fn parses_empty_bulk_string() {
        let (value, cursor) = deserializer(b"$0\r\n\r\n").unwrap();
        assert_eq!(value, RespValue::BulkString(Bytes::from_static(b"")));
        assert_eq!(cursor, 6);
    }

    #[test]
    fn parses_empty_array() {
        let (value, cursor) = deserializer(b"*0\r\n").unwrap();
        assert_eq!(value, RespValue::Array(vec![]));
        assert_eq!(cursor, 4);
    }

    #[test]
    fn parses_flat_array() {
        let (value, _cursor) = deserializer(b"*2\r\n$3\r\nfoo\r\n:1\r\n").unwrap();
        assert_eq!(
            value,
            RespValue::Array(vec![
                RespValue::BulkString(Bytes::from_static(b"foo")),
                RespValue::Integer(1),
            ])
        );
    }

    #[test]
    fn parses_nested_array_without_losing_earlier_elements() {
        // Mirrors the serializer's nested-array regression test — makes sure
        // the cursor threading through recursive parse_serialized_input calls
        // doesn't re-slice byte_resp and lose track of position.
        let (value, cursor) = deserializer(b"*2\r\n*2\r\n:1\r\n:2\r\n$2\r\nab\r\n").unwrap();
        assert_eq!(
            value,
            RespValue::Array(vec![
                RespValue::Array(vec![RespValue::Integer(1), RespValue::Integer(2)]),
                RespValue::BulkString(Bytes::from_static(b"ab")),
            ])
        );
        assert_eq!(cursor, 24);
    }

    #[test]
    fn rejects_empty_input() {
        let err = deserializer(b"").unwrap_err();
        assert_eq!(err, RespErrorResponse::Incomplete);
    }

    #[test]
    fn rejects_invalid_first_byte_even_when_too_short() {
        let err = deserializer(b"a").unwrap_err();
        assert_eq!(err, RespErrorResponse::InvalidRESPType);
    }

    #[test]
    fn rejects_unknown_type_byte() {
        let err = deserializer(b"@1\r\n").unwrap_err();
        assert_eq!(err, RespErrorResponse::InvalidRESPType);
    }

    #[test]
    fn rejects_non_numeric_length_prefix() {
        let err = deserializer(b"$abc\r\nxxx\r\n").unwrap_err();
        assert_eq!(err, RespErrorResponse::IncorrectTypeError);
    }

    #[test]
    fn rejects_bulk_string_missing_trailing_crlf() {
        let err = deserializer(b"$3\r\nabc").unwrap_err();
        assert_eq!(err, RespErrorResponse::Incomplete);
    }

    #[test]
    fn rejects_bulk_string_with_wrong_length_count() {
        // Length says 5 but only 3 data bytes precede the CRLF that's actually there.
        let err = deserializer(b"$5\r\nabc\r\n").unwrap_err();
        assert_eq!(err, RespErrorResponse::Incomplete);
    }

    #[test]
    fn reports_incomplete_when_only_type_byte_has_arrived() {
        // '+' alone is a valid Simple String type byte — nowhere near malformed,
        // just not enough of the stream has arrived yet.
        let err = deserializer(b"+").unwrap_err();
        assert_eq!(err, RespErrorResponse::Incomplete);
    }

    #[test]
    fn reports_incomplete_instead_of_panicking_on_truncated_array_element() {
        // Array claims 2 elements but only 1 has arrived. Regression test for the
        // raw `byte_resp[cursor]` indexing that used to panic here once `cursor`
        // ran past the end of the buffer mid-recursion.
        let err = deserializer(b"*2\r\n:1\r\n").unwrap_err();
        assert_eq!(err, RespErrorResponse::Incomplete);
    }

    #[test]
    fn rejects_bulk_string_length_exceeding_max() {
        // usize::MAX as the stated length — would overflow `data_start + count` and
        // `expected_data_end + 1` if not rejected up front. Must fail cleanly with
        // BulkStringTooLarge, not panic (debug: overflow-checks; release: silent wraparound
        // into an invalid slice range).
        let err = deserializer(b"$18446744073709551615\r\nx").unwrap_err();
        assert_eq!(err, RespErrorResponse::BulkStringTooLarge);
    }

    #[test]
    fn rejects_invalid_utf8_in_integer() {
        let err = deserializer(b":\xFF\r\n").unwrap_err();
        assert_eq!(err, RespErrorResponse::UtfError);
    }

    #[test]
    fn round_trips_through_serializer_and_deserializer() {
        use crate::serializer::serializer;
        use bytes::BytesMut;

        let original = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from_static(b"foo")),
            RespValue::Integer(-42),
            RespValue::Null,
            RespValue::Array(vec![RespValue::SimpleString(Bytes::from_static(b"OK"))]),
        ]);

        let mut buf = BytesMut::new();
        serializer(&original, &mut buf);

        let (parsed, cursor) = deserializer(&buf).unwrap();
        assert_eq!(parsed, original);
        assert_eq!(cursor, buf.len());
    }
}
