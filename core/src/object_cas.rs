use std::io;

pub trait ObjectCAS: Send + Sync {
    fn delete(&self, hash: &[u8]) -> impl Future<Output = io::Result<()>> + Send;

    fn exists(&self, hash: &[u8]) -> impl Future<Output = io::Result<bool>> + Send;

    fn get(&self, hash: &[u8]) -> impl Future<Output = io::Result<String>> + Send;

    /// Insert an Object into the CAS.
    /// Is not required to replace if exists.
    fn put(&self, hash: &[u8], data: &str) -> impl Future<Output = io::Result<()>> + Send;
}
