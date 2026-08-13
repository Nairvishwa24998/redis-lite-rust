use bytes::{BufMut, BytesMut};

use crate::constants::{
    ARRAY_FIRST_BYTE, BULK_STRING_FIRST_BYTE, CRLF, ERROR_FIRST_BYTE, INTEGER_FIRST_BYTE,
    NULL_BULK_STRING, SIMPLE_STRING_FIRST_BYTE,
};
use crate::resp::RespValue;




// We don't split or freeze here coz it would cause issues recursively in the case of arrays
// Instead we can freeze at the point where the serializer is called
pub fn serializer(data: &RespValue, buf: &mut BytesMut) {
    match data {
        // Could be a null bulkstring or null array. Setting to former for convenience
        RespValue::Null => generate_serialized_null(buf),
        RespValue::Error(err) => generate_serialized_non_bulk_string_result(err, buf, ERROR_FIRST_BYTE),
        RespValue::SimpleString(s) => generate_serialized_non_bulk_string_result(s, buf, SIMPLE_STRING_FIRST_BYTE),
        RespValue::BulkString(s) => generate_serialized_bulk_string(s, buf, BULK_STRING_FIRST_BYTE),
        RespValue::Integer(i) => {
            generate_serialized_non_bulk_string_result(&i.to_string().into_bytes(), buf, INTEGER_FIRST_BYTE)
        }
        RespValue::Array(arr) => {
            let element_count: usize = arr.len();
            // This is the array count bit
            generate_serialized_non_bulk_string_result(
                &element_count.to_string().into_bytes(),
                buf,
                ARRAY_FIRST_BYTE,
            );
            for resp_value in arr {
                serializer(resp_value, buf);
            }
        }
    }
    // // Split drains the buffer and transfers ownership to a new instance which we then freeze
    // // If we don't split, we freeze the buffer and end up consuming the original mutBytes itself
    // buf.split().freeze()
}

fn generate_serialized_null(buf: &mut BytesMut) {
    buf.put_slice(&NULL_BULK_STRING);
}

// Better to accept as [u8]
fn generate_serialized_non_bulk_string_result(data_bytes: &[u8], buf: &mut BytesMut, first_byte: u8) {
    buf.put_u8(first_byte);
    buf.put_slice(data_bytes);
    buf.put_slice(&CRLF);
}

fn generate_serialized_bulk_string(data_bytes: &[u8], buf: &mut BytesMut, first_byte: u8) {
    buf.put_u8(first_byte);
    buf.put_slice(&data_bytes.len().to_string().into_bytes());
    buf.put_slice(&CRLF);
    buf.put_slice(data_bytes);
    buf.put_slice(&CRLF);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn serialize(data: &RespValue) -> BytesMut {
        let mut buf = BytesMut::new();
        serializer(data, &mut buf);
        buf
    }

    #[test]
    fn serializes_null_as_null_bulk_string() {
        let buf = serialize(&RespValue::Null);
        assert_eq!(&buf[..], b"$-1\r\n");
    }

    #[test]
    fn serializes_simple_string() {
        let buf = serialize(&RespValue::SimpleString(Bytes::from_static(b"OK")));
        assert_eq!(&buf[..], b"+OK\r\n");
    }

    #[test]
    fn serializes_error() {
        let buf = serialize(&RespValue::Error(Bytes::from_static(b"oh no")));
        assert_eq!(&buf[..], b"-oh no\r\n");
    }

    #[test]
    fn serializes_bulk_string() {
        let buf = serialize(&RespValue::BulkString(Bytes::from_static(b"abc")));
        assert_eq!(&buf[..], b"$3\r\nabc\r\n");
    }

    #[test]
    fn serializes_empty_bulk_string() {
        let buf = serialize(&RespValue::BulkString(Bytes::from_static(b"")));
        assert_eq!(&buf[..], b"$0\r\n\r\n");
    }

    #[test]
    fn serializes_positive_integer() {
        let buf = serialize(&RespValue::Integer(123));
        assert_eq!(&buf[..], b":123\r\n");
    }

    #[test]
    fn serializes_negative_integer() {
        let buf = serialize(&RespValue::Integer(-5));
        assert_eq!(&buf[..], b":-5\r\n");
    }

    #[test]
    fn serializes_empty_array() {
        let buf = serialize(&RespValue::Array(vec![]));
        assert_eq!(&buf[..], b"*0\r\n");
    }

    #[test]
    fn serializes_flat_array() {
        let buf = serialize(&RespValue::Array(vec![
            RespValue::BulkString(Bytes::from_static(b"foo")),
            RespValue::Integer(1),
        ]));
        assert_eq!(&buf[..], b"*2\r\n$3\r\nfoo\r\n:1\r\n");
    }

    #[test]
    fn serializes_nested_array_without_losing_earlier_elements() {
        // Regression test for the recursion bug documented in progress.md:
        // draining the buffer inside a recursive call discarded everything
        // written before it (e.g. the outer array's header).
        let buf = serialize(&RespValue::Array(vec![
            RespValue::Array(vec![RespValue::Integer(1), RespValue::Integer(2)]),
            RespValue::BulkString(Bytes::from_static(b"ab")),
        ]));
        assert_eq!(&buf[..], b"*2\r\n*2\r\n:1\r\n:2\r\n$2\r\nab\r\n");
    }
}
