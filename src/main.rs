#![allow(unused_imports)]

pub mod cmd;
pub mod resp;
use self::cmd::CMD;
use self::resp::Frame;
use anyhow::anyhow;
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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
                    eprintln!("no bytes to read");
                    break;
                }

                let src = buffer.to_vec();
                let mut src = Cursor::new(src.as_slice());
                let frame = Frame::parse(&mut src).unwrap();
                let _ = handle_frame(&mut stream, frame).await;
            }
        });
    }

    Ok(())
}

async fn handle_frame(stream: &mut TcpStream, frame: Frame) -> anyhow::Result<()> {
    let command: anyhow::Result<CMD> = (&frame).try_into();

    if command.is_err() {
        let response = frame.serialize();
        if response.is_err() {
            let error = format!("{:?}", response);
            stream
                .write_all(
                    &Frame::Error(error)
                        .serialize()
                        .expect("error frame can't failed to serialize"),
                )
                .await?;
            return Err(response.unwrap_err());
        }
        let response = response.unwrap();

        stream.write_all(&response).await?;
        return Ok(());
    }

    match command.unwrap() {
        CMD::Ping => {
            stream
                .write_all(&Frame::Simple("PONG".to_string()).serialize()?)
                .await?;
        }
        CMD::Echo(string) => {
            stream
                .write_all(&Frame::Bulk(string.into()).serialize()?)
                .await?;
        }
        CMD::Set { .. } => panic!("unimplemented"),
        CMD::Get { .. } => panic!("unimplemented"),
    };

    Ok(())
}
