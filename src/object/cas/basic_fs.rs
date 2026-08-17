use std::{io, path::PathBuf};
use tokio::fs;
use treeup_core::object_cas::ObjectCAS;

pub struct BasicFS {
    root: PathBuf,
}

impl BasicFS {
    pub async fn create(root: PathBuf) -> io::Result<Self> {
        // Precreate all directories
        let mut tasks = Vec::new();
        for i in u8::MIN..=u8::MAX {
            tasks.push(tokio::spawn(fs::create_dir_all(
                root.join(hex::encode([i])),
            )));
        }

        for task in tasks {
            task.await.expect("tokio join error in basicfs")?;
        }

        Ok(Self { root })
    }

    fn path(&self, hash: &[u8]) -> PathBuf {
        let prefix = hex::encode(&hash[0..1]);
        let suffix = hex::encode(&hash[1..]);
        self.root.join(prefix).join(suffix)
    }
}

impl ObjectCAS for BasicFS {
    async fn get(&self, hash: &[u8]) -> io::Result<String> {
        fs::read_to_string(self.path(hash)).await
    }

    async fn exists(&self, hash: &[u8]) -> io::Result<bool> {
        fs::try_exists(self.path(hash)).await
    }

    async fn put(&self, hash: &[u8], data: &str) -> io::Result<()> {
        let path = self.path(hash);

        if fs::try_exists(&path).await? {
            return Ok(());
        }

        fs::write(path, data).await
    }

    async fn delete(&self, hash: &[u8]) -> io::Result<()> {
        fs::remove_file(self.path(hash)).await
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;
    #[tokio::test]
    async fn basic() {
        let root = TempDir::new().unwrap();
        let cas = BasicFS::create(root.path().to_path_buf()).await.unwrap();

        cas.put(b"TEST", "Test data").await.unwrap();
        assert_eq!(cas.get(b"TEST").await.unwrap(), "Test data");
        cas.delete(b"TEST").await.unwrap();

        assert!(cas.get(b"TEST").await.is_err());
    }
}
