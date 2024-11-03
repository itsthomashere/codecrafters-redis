#![allow(unused_imports)]

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
                let mut buffer = [0; 256];
                let byte_read = stream.read(&mut buffer).await.unwrap();
                if byte_read == 0 {
                    break;
                }

                stream.write_all("+PONG\r\n".as_bytes()).await.unwrap();
            }
        });
    }

    Ok(())
}
