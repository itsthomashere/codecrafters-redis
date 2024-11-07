use anyhow::anyhow;
use bytes::{Buf, Bytes};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;

pub(crate) const DIR_ARGS: &str = "--dir";
pub(crate) const DB_FILE_NAME_ARGS: &str = "--dbfilename";

#[derive(Debug, Clone)]
pub struct RDB {
    pub dir: PathBuf,
    pub file_name: String,
}

impl RDB {
    pub fn build() -> anyhow::Result<Self> {
        let mut args = std::env::args();
        // skip the program actual binary
        args.next().unwrap();
        let mut dir = PathBuf::default();
        let mut file_name = String::default();
        while let Some(val) = args.next() {
            match val.as_str() {
                DIR_ARGS => {
                    let arg = args.next().ok_or(anyhow!("expected path"))?;
                    dir = PathBuf::from(arg);
                }
                DB_FILE_NAME_ARGS => {
                    let arg = args.next().ok_or(anyhow!("expected filename"))?;
                    file_name = arg;
                }
                _ => return Err(anyhow!("invalid args")),
            }
        }

        if file_name.is_empty() {
            return Err(anyhow!("empty file name"));
        }
        if dir.as_os_str().is_empty() {
            return Err(anyhow!("empty directory"));
        }

        Ok(Self { dir, file_name })
    }

    pub async fn load(&self) -> anyhow::Result<DB> {
        //  validate path
        if !self.dir.is_dir() {
            return Err(anyhow!(
                "{} is not a directory",
                self.dir.to_str().unwrap_or("")
            ));
        }

        let mut file_path = self.dir.clone();
        file_path.set_file_name(&self.file_name);
        //  validate file
        if !file_path.is_file() {
            return Err(anyhow!(
                "file {} does not exists",
                file_path.to_str().unwrap_or("")
            ));
        }
        //  load into buffer
        let buffer = tokio::fs::read(file_path).await?;
        let mut cursor = Cursor::new(buffer.as_slice());
        //  parse the buffer
        let pool = DB::parse(&mut cursor)?;

        Ok(DB { pool })
    }
}

#[derive(Debug)]
pub struct DB {
    pool: HashMap<String, Bytes>,
}

impl DB {
    pub fn get(&self, key: impl AsRef<str>) -> Option<Bytes> {
        self.pool.get(key.as_ref()).cloned()
    }
    pub fn set(&mut self, key: String, data: Bytes) -> Option<Bytes> {
        self.pool.insert(key, data)
    }

    pub(crate) fn parse(buffer: &mut Cursor<&[u8]>) -> anyhow::Result<HashMap<String, Bytes>> {
        todo!()
    }
}

fn read_redis_header(src: &mut Cursor<&[u8]>) -> anyhow::Result<()> {
    let magic = std::str::from_utf8(&src.chunk()[0..5])?;
    if magic != "REDIS" {
        return Err(anyhow!("wrong magic string: {}", magic));
    }
    src.advance(5);

    std::str::from_utf8(&src.chunk()[0..4])?.parse::<i32>()?;
    src.advance(4);

    Ok(())
}
