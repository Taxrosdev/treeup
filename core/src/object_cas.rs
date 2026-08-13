use std::io;

pub trait ObjectCAS {
    fn get(hash: &str) -> impl Future<Output = io::Result<String>> + Send;

    /// Insert an Object into the CAS.
    /// Should not replace if exists.
    fn put(hash: &str, data: &str) -> impl Future<Output = io::Result<()>> + Send;

    fn delete(hash: &str) -> impl Future<Output = io::Result<()>> + Send;
}
