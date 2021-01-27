use crate::stable_storage_public::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use log;


pub mod stable_storage_public {
    #[async_trait::async_trait]
    /// A helper trait for small amount of durable metadata needed by the register algorithm
    /// itself. Again, it is only for AtomicRegister definition. StableStorage in unit tests
    /// is durable, as one could expect.
    pub trait StableStorage: Send + Sync {
        async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

        async fn get(&self, key: &str) -> Option<Vec<u8>>;
    }

    #[async_trait::async_trait]
    pub trait ConstStableStorage: Send + Sync {
        async fn put(&self, key: &str, value: &[u8]) -> Result<(), String>;

        async fn get(&self, key: &str) -> Option<Vec<u8>>;
    }
}

#[derive(Clone)]
pub struct BasicStableStorage {
    pub root: PathBuf,
}

impl BasicStableStorage {
    pub async fn new(root: PathBuf) -> Self {
        let mut meta_storage = PathBuf::new();
        meta_storage.push(root.clone());
        meta_storage.push("meta");
        tokio::fs::create_dir(meta_storage.clone()).await.unwrap_or_else(|_| ()); // TODO if it actually DOES exist DO NOT throw error
        Self{root: meta_storage}
    }

    pub async fn drop(&self, key: &str) -> Result<(), std::io::Error> {
        let fname = self.key_to_fname(key);
        tokio::fs::remove_file(fname).await
    }

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    fn key_to_fname(&self, key: &str) -> PathBuf {
        let basename = BasicStableStorage::calculate_hash(&key).to_string();
        let mut path = self.root.clone();
        path.push(PathBuf::from(basename));
        path
    }
}

#[async_trait::async_trait]
impl StableStorage for BasicStableStorage {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        let fname = self.key_to_fname(key);
        log::info!("PUT NA {}", key);

        log::debug!("[BasicStableStorage] put: trying to open file {:?}", &fname);
        match tokio::fs::File::create(fname.clone()).await {
                Ok(mut f) => {
                f.write_all(value).await.unwrap_or_else(|_| {log::error!("[fs] cannot write to {:?}", &fname);});
                Ok(())
            },
            Err(msg) => Err(msg.to_string())
        }
    }

    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let fname = self.key_to_fname(key);
        
        log::info!("GET NA {}", key);
        
        log::debug!("[BasicStableStorage] get: trying to open file {:?}", &fname);
        match tokio::fs::File::open(fname.clone()).await {
            Ok(mut f) => {
                let mut res = Vec::new();
                f.read_to_end(&mut res).await.unwrap_or_else(|_| {
                    log::error!("[fs] cannot read from {:?}", &fname);
                    return 0;
                });
                Some(res)
            },
            Err(_) => {
                None
            },
        }
    }
}

#[async_trait::async_trait]
impl ConstStableStorage for BasicStableStorage {
    async fn put(&self, key: &str, value: &[u8]) -> Result<(), String> {
        let fname = self.key_to_fname(key);
        log::debug!("[BasicStableStorage] put: trying to open file {:?}", &fname);
        match tokio::fs::File::create(fname.clone()).await {
                Ok(mut f) => {
                f.write_all(value).await.unwrap_or_else(|_| {log::error!("[fs] cannot write to {:?}", &fname);});
                Ok(())
            },
            Err(msg) => Err(msg.to_string())
        }
    }

    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let fname = self.key_to_fname(key);
        log::debug!("[BasicStableStorage] get: trying to open file {:?}", &fname);
        match tokio::fs::File::open(fname.clone()).await {
            Ok(mut f) => {
                let mut res = Vec::new();
                f.read_to_end(&mut res).await.unwrap_or_else(|_| {
                    log::error!("[fs] cannot read from {:?}", &fname);
                    return 0;
                });
                Some(res)
            },
            Err(_) => {
                None
            },
        }
    }
}
