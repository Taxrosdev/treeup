use std::{fs, io, sync::Arc};
use temp_dir::TempDir;
use treeup::{
    blob::BlobRef,
    object::{Deployable, cas::BasicFS},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let path = TempDir::new()?;
    let repo_path = TempDir::new()?;
    // You could store these in the same directory, but they are commonly seperated.
    let blobs_path = repo_path.path().join("blobs");
    let objects_path = repo_path.path().join("objects");
    // No objects are being created, so this should be empty.
    let cas = Arc::new(BasicFS::create(objects_path).await?);

    // Create a file that will become our blob.
    fs::write(path.child("blob"), "example file")?;

    // Finally, lets create our `BlobRef`, and then immediately `BlobRef::deploy` it somewhere.
    let blob = BlobRef::create(cas.clone(), &blobs_path, &path.child("blob")).await?;
    blob.deploy(cas, &blobs_path, &path.child("deployed_blob"))
        .await?;

    // Then we check that it's correct. This isn't actually necessary, it's just to demonstrate that
    // the contents will be the same.
    assert_eq!(
        fs::read_to_string(path.child("deployed_blob"))?,
        "example file".to_string()
    );

    Ok(())
}
