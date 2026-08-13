use bytes::Bytes;


// We use as Enum of Bytes for the most part
#[derive(Debug, PartialEq)]
pub enum RespValue {
    Null,
    Error(Bytes),
    SimpleString(Bytes),
    BulkString(Bytes),
    Integer(i64),
    Array(Vec<RespValue>),
}
