#![allow(unused_imports)]

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
                    Frame::Simple(string) => {
                        if &string.to_lowercase() == "ping" {
                            stream
                                .write_all(
                                    format!("{}", Frame::Simple("PONG".to_string())).as_bytes(),
                                )
                                .await
                                .unwrap();
                        }
                    }
                    Frame::Error(error) => {
                        stream
                            .write_all(format!("{}", Frame::Error(error)).as_bytes())
                            .await
                            .unwrap();
                    }
                    Frame::Interger(i) => {
                        stream
                            .write_all(format!("{}", Frame::Interger(i)).as_bytes())
                            .await
                            .unwrap();
                    }
                    Frame::Null => stream
                        .write_all(format!("{}", Frame::Null).as_bytes())
                        .await
                        .unwrap(),
                    Frame::Bulk(bytes) => {
                        stream
                            .write_all(format!("{}", Frame::Bulk(bytes)).as_bytes())
                            .await
                            .unwrap();
                    }
                    Frame::Array(arr) => {
                        let cmd = match &arr[0] {
                            Frame::Bulk(bytes) => std::str::from_utf8(bytes).unwrap(),
                            _ => panic!("bad request"),
                        };

                        match cmd.to_lowercase().as_str() {
                            "echo" => {
                                stream
                                    .write_all(format!("{}", arr[1]).as_bytes())
                                    .await
                                    .unwrap();
                            }
                            "ping" => stream
                                .write_all(
                                    format!("{}", Frame::Simple("PONG".to_string())).as_bytes(),
                                )
                                .await
                                .unwrap(),
                            _ => panic!("unhandled"),
                        }
                    }
                }
            }
        });
    }

    Ok(())
}
