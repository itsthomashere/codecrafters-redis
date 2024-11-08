pub mod cmd;
pub mod rdb;
pub mod resp;
use self::cmd::CMD;
use self::rdb::RDB;
use self::resp::Frame;
use bytes::Bytes;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rdb = RDB::build().ok();
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = tokio::net::TcpListener::bind("127.0.0.1:6379").await?;

    let database: Arc<Mutex<HashMap<String, Item>>> = Arc::new(Mutex::new(HashMap::default()));
    while let Ok((mut stream, _socket)) = listener.accept().await {
        let db = database.clone();
        let r_db = Arc::new(RwLock::new(rdb.clone()));
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
                while let Ok(frame) = Frame::parse(&mut src) {
                    let _ = handle_frame(&mut stream, frame, db.clone(), r_db.clone()).await;
                }
            }
        });
    }

    Ok(())
}

async fn handle_frame(
    stream: &mut TcpStream,
    frame: Frame,
    database: Arc<Mutex<HashMap<String, Item>>>,
    rdb: Arc<RwLock<Option<RDB>>>,
) -> anyhow::Result<()> {
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

    // create response variable to avoid hoding the mutex across
    // await point
    let response = match command.unwrap() {
        CMD::Ping => Frame::Simple("PONG".to_string()).serialize()?,
        CMD::Echo(string) => Frame::Bulk(string.into()).serialize()?,
        CMD::Set { key, value, expire } => {
            database
                .lock()
                .unwrap()
                .insert(key.clone(), Item { value, expire });

            Frame::Simple("OK".to_string()).serialize()?
        }
        CMD::Get { key } => {
            let value = database.lock().unwrap();
            let value = value
                .get(&key)
                .take()
                .map(|val| {
                    if !val.expired() {
                        Frame::Bulk(val.value.clone())
                    } else {
                        Frame::Null
                    }
                })
                .unwrap_or(Frame::Null);

            value.serialize()?
        }
        CMD::Config { dir, file_name } => {
            let mut response: Vec<Frame> = Vec::new();
            let rdb = rdb.read().unwrap().clone().expect("should be set");
            if dir {
                let dir: Bytes = Bytes::from(rdb.dir.to_str().unwrap_or_default().to_string());
                response.push(Frame::Bulk("dir".into()));
                response.push(Frame::Bulk(dir))
            }
            if file_name {
                let dir: Bytes = Bytes::from(rdb.file_name);
                response.push(Frame::Bulk("dbfilename".into()));
                response.push(Frame::Bulk(dir))
            }

            let value = Frame::Array(response);

            value.serialize()?
        }
    };

    stream.write_all(&response).await?;

    Ok(())
}

#[derive(Debug, Hash)]
pub struct Item {
    value: Bytes,
    expire: Option<(Instant, Duration)>,
}

impl Item {
    pub fn expired(&self) -> bool {
        match self.expire {
            Some((start, timeout)) => start.clone().elapsed() >= timeout,
            None => false,
        }
    }
}
