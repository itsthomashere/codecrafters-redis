#![allow(unused_imports)]

pub mod cmd;
pub mod resp;
use self::resp::Frame;
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = tokio::net::TcpListener::bind("127.0.0.1:6379").await?;

    while let Ok((mut stream, _socket)) = listener.accept().await {
        tokio::spawn(async move {
            loop {
                let mut buffer = [0; 521];
                let byte_read = stream.read(&mut buffer).await.unwrap();
                if byte_read == 0 {
                    break;
                }

                let src = buffer.to_vec();
                let mut src = Cursor::new(src.as_slice());

                let frame = Frame::parse(&mut src).unwrap();

                match frame {
                    Frame::Simple(ref string) => {
                        if string.to_lowercase() == "ping" {
                            let response = frame.serialize();
                            if response.is_err() {
                                eprintln!("failed to serialize message: {:?}", response);
                                continue;
                            }
                            let response = response.unwrap();

                            stream.write_all(&response).await.unwrap();
                        } else {
                            stream
                                .write_all(
                                    &frame
                                        .serialize()
                                        .expect("simple string should not fail to serialize"),
                                )
                                .await
                                .unwrap();
                        }
                    }
                    Frame::Error(_) => {
                        let response = frame.serialize();
                        if response.is_err() {
                            continue;
                        }
                        let response = response.unwrap();

                        stream.write_all(&response).await.unwrap();
                    }
                    Frame::Interger(_) => {
                        let response = frame.serialize();
                        if response.is_err() {
                            eprintln!("failed to serialize message: {:?}", response);
                            continue;
                        }
                        let response = response.unwrap();

                        stream.write_all(&response).await.unwrap_or_default();
                    }
                    Frame::Null => {
                        let response = frame.serialize();
                        if response.is_err() {
                            eprintln!("failed to serialize message: {:?}", response);
                            continue;
                        }
                        let response = response.unwrap();

                        stream.write_all(&response).await.unwrap_or_default();
                    }
                    Frame::Bulk(_) => {
                        let response = frame.serialize();
                        if response.is_err() {
                            eprintln!("failed to serialize message: {:?}", response);
                            continue;
                        }
                        let response = response.unwrap();

                        stream.write_all(&response).await.unwrap_or_default();
                    }
                    Frame::Array(arr) => {
                        let cmd = match &arr[0] {
                            Frame::Bulk(bytes) => {
                                let message = std::str::from_utf8(bytes);
                                if message.is_err() {
                                    continue;
                                }
                                message.unwrap_or_default()
                            }
                            _ => panic!("bad request"),
                        };

                        match cmd.to_lowercase().as_str() {
                            "echo" => {
                                if arr.len() < 2 {
                                    eprintln!("Array message len not enough");
                                }

                                let response = arr[1].serialize();
                                if response.is_err() {
                                    eprintln!("failed to serialize message: {:?}", response);
                                    continue;
                                }
                                let response = response.unwrap(); // checked
                                                                  //
                                stream.write_all(&response).await.unwrap_or_default();
                            }
                            "ping" => stream
                                .write_all(
                                    &Frame::Simple("PONG".to_string())
                                        .serialize()
                                        .expect("valid simple frame"),
                                )
                                .await
                                .unwrap_or_default(),
                            _ => stream
                                .write_all(
                                    &Frame::Error(format!("{cmd} is not a command"))
                                        .serialize()
                                        .expect("simple error cannot fail to serialize"),
                                )
                                .await
                                .unwrap_or_default(),
                        }
                    }
                }
            }
        });
    }

    Ok(())
}
