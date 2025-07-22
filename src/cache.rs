use std::path::PathBuf;
#[cfg(feature = "native")]
use tokio::fs;
#[cfg(feature = "native")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "worker")]
use worker::kv::KvStore;

#[cfg(feature = "native")]
pub struct ImageCache {
    cache_dir: PathBuf,
}

#[cfg(feature = "worker")]
pub struct ImageCache {
    kv: KvStore,
}

#[cfg(feature = "native")]
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

#[cfg(feature = "worker")]
impl ImageCache {
    pub fn new_kv(kv: KvStore) -> Self {
        Self { kv }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.kv.get(key).bytes().await.ok()
    }

    pub async fn put(&mut self, key: String, data: Vec<u8>) {
        let _ = self
            .kv
            .put(&key, data)
            .expiration_ttl(86400)
            .execute()
            .await;
    }
}
