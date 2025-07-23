use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct ImageCache {
    cache_dir: PathBuf,
}

impl ImageCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let file_path = self.cache_dir.join(key);

        if !file_path.exists() {
            return None;
        }

        match fs::File::open(&file_path).await {
            Ok(mut file) => {
                let mut contents = Vec::new();
                match file.read_to_end(&mut contents).await {
                    Ok(_) => Some(contents),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    pub async fn put(&mut self, key: String, data: Vec<u8>) {
        let file_path = self.cache_dir.join(&key);

        if let Ok(mut file) = fs::File::create(&file_path).await {
            let _ = file.write_all(&data).await;
        }
    }
}
