use crate::resp::Frame;
use anyhow::anyhow;
use bytes::Bytes;
use std::time::{self, Duration, SystemTime};

#[derive(Debug, Eq, PartialEq)]
pub enum CMD {
    Ping,
    Echo(String),
    Set {
        key: String,
        value: Bytes,
        expire: Option<(time::SystemTime, Duration)>,
    },
    Get {
        key: String,
    },
}

impl TryFrom<&Frame> for CMD {
    type Error = anyhow::Error;

    fn try_from(value: &Frame) -> Result<Self, Self::Error> {
        match value {
            Frame::Simple(string) => {
                if string.to_lowercase() == "ping" {
                    Ok(Self::Ping)
                } else {
                    Err(anyhow!("invalid command"))
                }
            }
            Frame::Error(_) => Err(anyhow!("error frame can't be command")),
            Frame::Interger(_) => Err(anyhow!("integer frame can't be command")),
            Frame::Null => Err(anyhow!("null frame can't be command")),
            Frame::Bulk(bytes) => match std::str::from_utf8(bytes)?.to_lowercase().as_str() {
                "ping" => Ok(Self::Ping),
                _ => Err(anyhow!("invalid command")),
            },
            Frame::Array(arr) => from_vec_cmd(arr),
        }
    }
}

fn from_vec_cmd(arr: &[Frame]) -> anyhow::Result<CMD> {
    println!("{:?}", arr);
    if arr.is_empty() {
        return Err(anyhow!("empty array"));
    }
    let command_name_lc = std::str::from_utf8(&arr[0].into_bytes()?)?.to_owned();

    match command_name_lc.to_lowercase().as_str() {
        "ping" => Ok(CMD::Ping),
        "echo" => {
            if arr.len() < 2 {
                return Ok(CMD::Echo("".into()));
            }
            let message = std::str::from_utf8(&arr[1].into_bytes()?)?.to_string();

            Ok(CMD::Echo(message))
        }
        "get" => {
            if arr.len() < 2 {
                return Err(anyhow!("get command without key"));
            };

            let key = std::str::from_utf8(&arr[1].into_bytes()?)?.to_string();
            Ok(CMD::Get { key })
        }
        "set" => {
            if arr.len() < 3 {
                return Err(anyhow!("set command without key, or value"));
            }
            let key = std::str::from_utf8(&arr[1].into_bytes()?)?.to_string();
            let value = arr[2].into_bytes()?;

            let mut expire = None;
            // this mean we have expiry date
            if arr.len() > 4 {
                let px = std::str::from_utf8(&arr[3].into_bytes()?)?
                    .to_string()
                    .to_lowercase();
                if px.as_str() != "px" {
                    return Err(anyhow!("Invalid time syntax"));
                }

                let timeout: u64 = std::str::from_utf8(&arr[4].into_bytes()?)?.parse()?;

                expire = Some((SystemTime::now(), Duration::from_millis(timeout)))
            }
            Ok(CMD::Set { key, value, expire })
        }
        _ => Err(anyhow!("unimplemented command")),
    }
}
