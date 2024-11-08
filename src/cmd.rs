use crate::resp::Frame;
use anyhow::anyhow;
use bytes::Bytes;
use std::time::{self, Duration, Instant};

#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CMD {
    Ping,
    Echo(String),
    Set {
        key: String,
        value: Bytes,
        expire: Option<(time::Instant, Duration)>,
    },
    Get {
        key: String,
    },
    Config {
        dir: bool,
        file_name: bool,
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

                expire = Some((Instant::now(), Duration::from_millis(timeout)))
            }
            Ok(CMD::Set { key, value, expire })
        }
        "config" => {
            if arr.len() < 3 {
                return Err(anyhow!("not enough arguments for config"));
            };
            if std::str::from_utf8(&arr[1].into_bytes()?)?
                .to_lowercase()
                .as_str()
                != "get"
            {
                return Err(anyhow!("only supported CONFIG GET atm"));
            }

            let mut dir = false;
            let mut file_name = false;
            let iter = arr.iter().skip(2);

            for value in iter {
                let value = std::str::from_utf8(&value.into_bytes()?)?.to_lowercase();
                match value.as_str() {
                    "dir" => dir = true,
                    "dbfilename" => file_name = true,
                    _ => return Err(anyhow!("unimplemented config")),
                }
            }

            Ok(CMD::Config { dir, file_name })
        }
        _ => Err(anyhow!("unimplemented command")),
    }
}
