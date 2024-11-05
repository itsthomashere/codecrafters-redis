use bytes::Bytes;

#[derive(Debug, Eq, PartialEq)]
pub enum CMD {
    Ping,
    Echo(String),
    Set { key: String, value: Bytes },
    Get { key: String },
}
