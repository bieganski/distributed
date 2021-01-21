pub mod sectors_manager_public {
    use std::sync::Arc;
    use crate::{SectorIdx, SectorVec};
    use std::path::PathBuf;
    use std::io::SeekFrom;
    use tokio::fs::OpenOptions;
    use tokio::prelude::*;
    use bincode;
    
    #[async_trait::async_trait]
    pub trait SectorsManager: Send + Sync {
        /// Returns 4096 bytes of sector data by index.
        async fn read_data(&self, idx: SectorIdx) -> SectorVec;

        /// Returns timestamp and write rank of the process which has saved this data.
        /// Timestamps and ranks are relevant for atomic register algorithm, and are described
        /// there.
        async fn read_metadata(&self, idx: SectorIdx) -> (u64, u8);

        /// Writes a new data, along with timestamp and write rank to some sector.
        async fn write(&self, idx: SectorIdx, sector: &(SectorVec, u64, u8));
    }

    /// Path parameter points to a directory to which this method has exclusive access.
    pub fn build_sectors_manager(path: PathBuf) -> Arc<dyn SectorsManager> {
        Arc::new(BasicSectorsManager{path})
    }

    pub struct BasicSectorsManager {
        path: PathBuf,
    }

    impl BasicSectorsManager {
        fn filepath(&self, idx: SectorIdx ) -> PathBuf {
            let mut tmp = self.path.clone();
            tmp.push(idx.to_string());
            tmp
        }
    }
    
    #[async_trait::async_trait]
    impl SectorsManager for BasicSectorsManager {

        async fn read_metadata(&self, idx: SectorIdx) -> (u64, u8) {
            let path = self.filepath(idx);
            let file = tokio::fs::File::open(&path).await;
            
            let mut res = vec![];

            match file {
                Ok(mut file) => {
                    file.seek(SeekFrom::Start(4096)).await.unwrap();
                    file.read_to_end(&mut res).await.unwrap();
                    bincode::deserialize(&res).unwrap()
                },
                Err(_) => {
                    (0_u64, 0_u8)
                },
            }
        }

        async fn read_data(&self, idx: SectorIdx) -> SectorVec {
            let path = self.filepath(idx);
            let file = tokio::fs::File::open(&path).await;
            
            let mut tmp : [u8; 4096] = [0; 4096];
            match file {
                Ok(mut file) => {
                    file.read_exact(&mut tmp).await.unwrap();
                },
                Err(_) => {},
            }

            let mut res = Vec::new();
            res.extend_from_slice(&tmp);
            SectorVec(res)
        }

        async fn write(&self, idx: SectorIdx, sector: &(SectorVec, u64, u8)) {
            let path = self.filepath(idx);
            let file = OpenOptions::new()
                .write(true)
                .open(&path)
                .await;
            
            let mut file = match file {
                Ok(file) => {
                    file
                },
                Err(_) => {
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&path)
                        .await.unwrap()
                },
            };
            file.seek(SeekFrom::Start(0)).await.unwrap();
            let SectorVec(data) = &sector.0;
            file.write_all(&data).await.unwrap();
            let meta = (sector.1, sector.2);
            file.write_all(&bincode::serialize(&meta).unwrap()).await.unwrap();
        }
    }
}
