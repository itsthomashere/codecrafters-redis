use anyhow::anyhow;
use bytes::{Buf, Bytes};
use std::io::Cursor;

#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum Frame {
    Simple(String),
    Error(String),
    Interger(i64),
    Null,
    Bulk(Bytes),
    Array(Vec<Frame>),
}

impl Frame {
    pub fn parse(src: &mut Cursor<&[u8]>) -> anyhow::Result<Frame> {
        match get_u8(src)? {
            b'+' => {
                let line = get_line(src)?;
                let message = String::from_utf8(line.to_vec())?;

                Ok(Self::Simple(message))
            }
            b'-' => {
                let line = get_line(src)?;
                let error = String::from_utf8(line.to_vec())?;

                Ok(Self::Error(error))
            }
            b':' => Ok(Self::Interger(get_decimal(src)?)),
            b'$' => {
                if peek_u8(src)? == b'-' {
                    let line = get_line(src)?;
                    if line != b"-1" {
                        return Err(anyhow!("invalid bulk message"));
                    }

                    Ok(Frame::Null)
                } else {
                    let len = get_decimal(src)? as usize;
                    let byte_read = len + 2; // include "\r\n": 2 bytes
                    let result = Bytes::copy_from_slice(&src.chunk()[..len]);

                    skip(src, byte_read)?;

                    Ok(Frame::Bulk(result))
                }
            }
            b'*' => {
                let len = get_decimal(src)?;

                let mut result: Vec<Frame> = Vec::new();
                for _ in 0..len {
                    result.push(Frame::parse(src)?);
                }

                Ok(Frame::Array(result))
            }
            _ => Err(anyhow!("unknown protocol")),
        }
    }

    pub fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        match self {
            Frame::Simple(string) => Ok(format!("+{}\r\n", string).into_bytes()),
            Frame::Error(error) => Ok(format!("-{}\r\n", error).into_bytes()),
            Frame::Interger(int) => Ok(format!(":{}\r\n", int).into_bytes()),
            Frame::Null => Ok(b"$-1\r\n".to_vec()),
            Frame::Bulk(values) => {
                Ok(
                    format!("${}\r\n{}\r\n", values.len(), std::str::from_utf8(values)?)
                        .into_bytes(),
                )
            }
            Frame::Array(arr) => {
                let len = arr.len();
                let mut result = format!("*{}\r\n", len).into_bytes();

                for i in arr {
                    result.append(&mut i.serialize()?);
                }

                Ok(result)
            }
        }
    }

    pub fn into_bytes(&self) -> anyhow::Result<Bytes> {
        match self {
            Frame::Simple(string) => Ok(string.clone().into()),
            Frame::Error(string) => Ok(string.clone().into()),
            Frame::Interger(_) => Err(anyhow!("integer type can't convert to a bytes")),
            Frame::Null => Err(anyhow!("null can't convert to bytes")),
            Frame::Bulk(bytes) => Ok(bytes.clone()),
            Frame::Array(_) => Err(anyhow!("array -> bytes: unimplemented conversion")),
        }
    }
}

pub(crate) fn get_u8(src: &mut Cursor<&[u8]>) -> anyhow::Result<u8> {
    if !src.has_remaining() {
        return Err(anyhow!("no more bytes"));
    }

    Ok(src.get_u8())
}

pub(crate) fn peek_u8(src: &mut Cursor<&[u8]>) -> anyhow::Result<u8> {
    if !src.has_remaining() {
        return Err(anyhow!("no more bytes"));
    }

    Ok(src.chunk()[0])
}

fn get_decimal(src: &mut Cursor<&[u8]>) -> anyhow::Result<i64> {
    let line = std::str::from_utf8(get_line(src)?)?;

    Ok(line.parse()?)
}

fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> anyhow::Result<&'a [u8]> {
    let start = src.position() as usize;
    let end = src.get_ref().len() - 1;

    for i in start..end {
        let arr = src.get_ref();

        if arr[i] == b'\r' && arr[i + 1] == b'\n' {
            // advance to the byte after '\n'
            src.set_position((i + 2) as u64);

            // were at '\r', so exclude the i'th byte
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(anyhow!("could not find end delimiters"))
}

pub(crate) fn skip(src: &mut Cursor<&[u8]>, n: usize) -> anyhow::Result<()> {
    if src.remaining() < n {
        return Err(anyhow!("no more bytes"));
    }

    src.advance(n);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_string() {
        let mut src = Cursor::new("+hello\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Simple("hello".to_string());

        assert_eq!(expected_frame, output_frame)
    }

    #[test]
    fn parse_error() {
        let mut src = Cursor::new("-error\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Error("error".to_string());

        assert_eq!(expected_frame, output_frame)
    }

    #[test]
    fn parse_integer_positive() {
        let mut src = Cursor::new(":100\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Interger(100);

        assert_eq!(expected_frame, output_frame);

        let mut src = Cursor::new(":+100\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Interger(100);

        assert_eq!(expected_frame, output_frame)
    }

    #[test]
    fn parse_integer_negative() {
        let mut src = Cursor::new(":-100\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Interger(-100);

        assert_eq!(expected_frame, output_frame);
    }

    #[test]
    fn parse_bulk_non_empty() {
        let mut src = Cursor::new("$3\r\nhey\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Bulk(Bytes::from("hey"));

        assert_eq!(expected_frame, output_frame);
    }

    #[test]
    fn parse_bulk_empty() {
        let mut src = Cursor::new("$-1\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Null;

        assert_eq!(expected_frame, output_frame);
    }

    #[test]
    #[should_panic]
    fn failed_with_invalid_seq() {
        let mut src = Cursor::new("paosidjf;lkjewqpoi".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
    }

    #[test]
    fn parse_array() {
        let mut src = Cursor::new("*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n".as_bytes());

        let output_frame = Frame::parse(&mut src).expect("valid byte sequence");
        let expected_frame = Frame::Array(vec![
            Frame::Bulk(Bytes::from("ECHO")),
            Frame::Bulk(Bytes::from("hey")),
        ]);

        assert_eq!(expected_frame, output_frame);
    }
}
