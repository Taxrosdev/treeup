//! Test reference implementations of `ObjectCAS`

use std::io;

use temp_dir::TempDir;
use treeup::object::{
    cas::{basic_fs::BasicFS, packfile::PackfileCAS},
    error::Result,
};
use treeup_core::object_cas::ObjectCAS;

#[tokio::test]
async fn cas_basicfs() -> Result<()> {
    let root = TempDir::new()?;
    let create_cas = || BasicFS::create(root.path().into());
    test_cas(create_cas).await
}

#[tokio::test]
async fn cas_packfile() -> Result<()> {
    let root = TempDir::new()?;
    let create_cas = || PackfileCAS::create(root.path().into());
    test_cas(create_cas).await
}

async fn test_cas<F, Fut, C: ObjectCAS>(create_cas: F) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = io::Result<C>>,
{
    let cas = create_cas().await?;

    cas.put(b"hello", "world").await?;
    assert_eq!(cas.get(b"hello").await?, "world");
    assert!(cas.exists(b"hello").await?);

    // Recreate CAS
    drop(cas);
    let cas = create_cas().await?;

    // Assert CAS data persisted
    assert_eq!(cas.get(b"hello").await?, "world");
    assert!(cas.exists(b"hello").await?);

    // Assert deletion works
    cas.delete(b"hello").await?;
    assert!(!cas.exists(b"hello").await?);
    assert!(cas.get(b"hello").await.is_err());

    Ok(())
}
