use std::{fs, io};
use temp_dir::TempDir;
use treeup::{blob::BlobRef, downloader::ReqwestDownloader, object::Deployable, repo::Repo};

#[tokio::main]
async fn main() -> io::Result<()> {
    let cwd = TempDir::new()?;
    let repo_path = TempDir::new()?;
    // You could store these in the same directory, but they are commonly seperated.
    let blobs_path = repo_path.path().join("blobs");
    let objects_path = repo_path.path().join("objects");

    // Prepare `Downloader`. As this is a local only `Repo`, we're just going to leave this with
    // empty strings for now.
    let downloader = ReqwestDownloader::new("", "");

    // Create a `Repo`
    let repo = Repo {
        objects_path,
        blobs_path,
        downloader: Box::new(downloader),
    };

    // Create a file that will become our blob.
    fs::write(cwd.child("blob"), "example file")?;

    // Finally, lets create our `BlobRef`, and then immediately `BlobRef::deploy` it somewhere.
    let blob = BlobRef::create(&repo, &cwd.child("blob")).await?;
    blob.deploy(&repo, &cwd.child("deployed_blob")).await?;

    // Then we check that it's correct. This isn't actually nessecery, it's just to demonstrate that
    // the contents will be the same.

    assert_eq!(
        fs::read_to_string(cwd.child("deployed_blob"))?,
        "example file".to_string()
    );

    Ok(())
}
