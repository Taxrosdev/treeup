use memmap2::{MmapMut, RemapOptions};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
};
use tokio::{fs, sync::RwLock};
use treeup_core::object_cas::ObjectCAS;

use crate::utils::atomic::{atomic_rename, atomic_rename_blocking};

const PACKFILE_MAGIC: [u8; 4] = *b"PACK";
const MAX_PACKFILE_INSERT: usize = 2048;

pub struct Packfile {
    index: HashMap<Vec<u8>, PackfileIndex>,
    index_path: PathBuf,
    data: MmapMut,
    data_file: File,
}

impl Drop for Packfile {
    fn drop(&mut self) {
        let tmp_file = self.index_path.with_extension("index.tmp");
        let mut data = Vec::new();
        data.extend_from_slice(&PACKFILE_MAGIC);

        // It should NEVER be read whilst `Drop` is being called, hence, this is entirely safe.
        for (hash, index) in &self.index {
            data.push(hash.len().try_into().expect("Hash is above 256 bytes"));
            data.extend_from_slice(hash);

            data.extend_from_slice(&index.start.to_le_bytes());
            data.extend_from_slice(&index.len.to_le_bytes());
        }

        std::fs::write(&tmp_file, data).expect("packfile fs error");
        atomic_rename_blocking(&tmp_file, &self.index_path)
            .expect("atomic rename failure on packfile drop");
    }
}

impl Packfile {
    pub async fn init(index_path: PathBuf, data_path: &Path) -> io::Result<Self> {
        let data_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(data_path)?;

        Ok(Self {
            index: PackfileIndex::read_indexes(&index_path).await?,
            index_path,
            data: Self::load_data(&data_file)?,
            data_file,
        })
    }

    fn load_data(file: &File) -> io::Result<MmapMut> {
        unsafe { MmapMut::map_mut(file) }
    }
}

pub struct PackfileIndex {
    pub(crate) start: u64,
    pub(crate) len: u64,
}

impl PackfileIndex {
    pub async fn read_indexes(path: &Path) -> io::Result<HashMap<Vec<u8>, PackfileIndex>> {
        if !fs::try_exists(&path).await? {
            return Ok(HashMap::new());
        }

        let file = fs::read(path).await?;

        Self::parse_indexes(&file)
    }

    pub fn parse_indexes(raw: &[u8]) -> io::Result<HashMap<Vec<u8>, PackfileIndex>> {
        let raw = &mut raw.iter().copied();
        let magic: Vec<u8> = raw.take(4).collect();
        if magic != PACKFILE_MAGIC {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let mut index = HashMap::new();

        while let Some(hash_length) = raw.next() {
            let hash: Vec<_> = raw.take(hash_length as usize).collect();
            let start_raw: Vec<_> = raw.take(8).collect();
            let start = u64::from_le_bytes(
                start_raw
                    .try_into()
                    .map_err(|_| io::ErrorKind::InvalidData)?,
            );
            let len_raw: Vec<_> = raw.take(8).collect();
            let len =
                u64::from_le_bytes(len_raw.try_into().map_err(|_| io::ErrorKind::InvalidData)?);

            index.insert(hash, PackfileIndex { start, len });
        }

        Ok(index)
    }
}

/// Requires Packfile compatible Downloaders
pub struct PackfileCAS {
    root: PathBuf,
    packfiles: HashMap<u8, RwLock<Packfile>>,
}

impl PackfileCAS {
    pub async fn create(root: PathBuf) -> io::Result<Self> {
        // Precreate all directories and packfiles
        let mut packfiles = HashMap::new();
        // TODO: This could be concurrent
        for i in u8::MIN..=u8::MAX {
            let path = root.join(hex::encode([i]));
            tokio::fs::create_dir_all(&path).await?;

            packfiles.insert(
                i,
                RwLock::new(
                    Packfile::init(path.join("packfile.idx"), &path.join("packfile")).await?,
                ),
            );
        }

        Ok(Self { root, packfiles })
    }

    fn path(&self, hash: &[u8]) -> PathBuf {
        let prefix = hex::encode(&hash[0..1]);
        let suffix = hex::encode(&hash[1..]);
        self.root.join(prefix).join(suffix)
    }
}

impl ObjectCAS for PackfileCAS {
    async fn get(&self, hash: &[u8]) -> io::Result<String> {
        let packfile = &self.packfiles[&hash[0]].read().await;
        match packfile.index.get(&hash[1..]) {
            Some(index) => {
                let start = index.start as usize;
                let end = (index.start + index.len) as usize;
                let raw = &packfile.data[start..end];
                String::from_utf8(raw.to_vec())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            }
            None => fs::read_to_string(self.path(hash)).await,
        }
    }

    async fn exists(&self, hash: &[u8]) -> io::Result<bool> {
        let packfile = &self.packfiles[&hash[0]].read().await;
        match packfile.index.get(&hash[1..]) {
            Some(_) => Ok(true),
            None => fs::try_exists(self.path(hash)).await,
        }
    }

    async fn put(&self, hash: &[u8], data: &str) -> io::Result<()> {
        if data.len() < MAX_PACKFILE_INSERT {
            let mut packfile = self.packfiles[&hash[0]].write().await;

            if packfile.index.contains_key(&hash[1..]) {
                return Ok(());
            }

            let start = packfile.data.len();
            let end = packfile.data.len() + data.len();

            packfile.data_file.set_len(end as u64)?;
            cfg_select!(
                target_os = "linux" => unsafe { packfile.data.remap(end, RemapOptions::new().may_move(true))? },
                _ => {
                    *packfile_data = Packfile::load_data(&packfile.data_file)?;
                }
            );
            packfile.data[start..end].copy_from_slice(data.as_bytes());

            packfile.index.insert(
                hash[1..].to_vec(),
                PackfileIndex {
                    start: start as u64,
                    len: data.len() as u64,
                },
            );

            Ok(())
        } else {
            let path = self.path(hash);
            if fs::try_exists(&path).await? {
                return Ok(());
            }

            let tmp_path = path.with_extension("tmp");
            fs::write(&tmp_path, data).await?;
            atomic_rename(tmp_path, path).await
        }
    }

    async fn delete(&self, hash: &[u8]) -> io::Result<()> {
        let path = self.path(hash);
        if fs::try_exists(&path).await? {
            fs::remove_file(path).await?;
        }

        let mut packfile = self.packfiles[&hash[0]].write().await;
        packfile.index.remove(&hash[1..]);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    #[tokio::test]
    async fn basic() {
        let root = TempDir::new().unwrap();
        let cas = PackfileCAS::create(root.path().to_path_buf())
            .await
            .unwrap();

        cas.put(b"TEST", "Test data").await.unwrap();
        assert_eq!(cas.get(b"TEST").await.unwrap(), "Test data");
        cas.delete(b"TEST").await.unwrap();

        assert!(cas.get(b"TEST").await.is_err());
    }
}
