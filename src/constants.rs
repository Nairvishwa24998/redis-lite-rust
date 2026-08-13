use bytes::Bytes;

pub const CRLF: Bytes = Bytes::from_static(b"\r\n");
pub const NULL_BULK_STRING: Bytes = Bytes::from_static(b"$-1\r\n");
pub const NULL_ARRAY: Bytes = Bytes::from_static(b"*-1\r\n");
pub const SIMPLE_STRING_FIRST_BYTE: u8 = b'+';
pub const BULK_STRING_FIRST_BYTE: u8 = b'$';
pub const ARRAY_FIRST_BYTE: u8 = b'*';
pub const INTEGER_FIRST_BYTE: u8 = b':';
pub const ERROR_FIRST_BYTE: u8 = b'-';
pub const CRLF_BYTES_OFFSET: usize = 2;
// Mirrors real Redis's `proto-max-bulk-len` default. Rejecting anything past this up front
// keeps `data_start + count` and friends nowhere near usize overflow, and avoids blocking
// forever waiting for bytes a well-behaved client would never send this many of anyway.
pub const MAX_BULK_STRING_LEN: usize = 512 * 1024 * 1024;
pub const REDIS_DEFAULT_PORT: u16 = 6379;
pub const REDIS_DEFAULT_URL: &str = "127.0.0.1:6379";
pub const BUFFER_PER_POLL_CALL: usize = 1024;
pub const SERVER_TOKEN: usize = 0;
