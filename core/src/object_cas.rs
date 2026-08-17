use std::io;

pub trait ObjectCAS: Send + Sync {
    fn get(&self, hash: &[u8]) -> impl Future<Output = io::Result<String>> + Send;

    fn exists(&self, hash: &[u8]) -> impl Future<Output = io::Result<bool>> + Send;

    /// Insert an Object into the CAS.
    /// Is not required to replace if exists.
    fn put(&self, hash: &[u8], data: &str) -> impl Future<Output = io::Result<()>> + Send;

    fn delete(&self, hash: &[u8]) -> impl Future<Output = io::Result<()>> + Send;
}
