use anyhow::anyhow;
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
}
