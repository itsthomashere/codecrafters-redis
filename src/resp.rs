use anyhow::anyhow;
use bytes::{Buf, Bytes};
use std::fmt::Display;
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

impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Simple(string) => write!(f, "+{string}\r\n"),
            Frame::Error(error) => write!(f, "-{error}\r\n"),
            Frame::Interger(int) => write!(f, ":{int}\r\n"),
            Frame::Null => write!(f, "$-1\r\n"),
            Frame::Bulk(bytes) => {
                let len = bytes.len();
                let string = std::str::from_utf8(bytes).unwrap();
                write!(f, "${len}\r\n{string}\r\n")
            }
            Frame::Array(arr) => {
                let mut output = String::from("");
                output.push_str(&format!("*{}\r\n", arr.len()));

                for e in arr {
                    output.push_str(&format!("{}\r\n", e))
                }
                write!(f, "{output}")
            }
        }
    }
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
                    println!("{}", len);
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
}

fn get_u8(src: &mut Cursor<&[u8]>) -> anyhow::Result<u8> {
    if !src.has_remaining() {
        return Err(anyhow!("no more bytes"));
    }

    Ok(src.get_u8())
}

fn peek_u8(src: &mut Cursor<&[u8]>) -> anyhow::Result<u8> {
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

fn skip(src: &mut Cursor<&[u8]>, n: usize) -> anyhow::Result<()> {
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
