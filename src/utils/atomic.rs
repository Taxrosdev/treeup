use nix::fcntl::{AT_FDCWD, RenameFlags, renameat2};
use std::{io, path::PathBuf};

/// Atomic renames using `renameat2`
///
/// # Errors
/// From the underlying filesystem syscall.
pub async fn atomic_rename(old_path: PathBuf, new_path: PathBuf) -> io::Result<()> {
    tokio::task::spawn_blocking(move || atomic_rename_blocking(old_path, new_path))
        .await
        .expect("internal panic on rename")?;

    Ok(())
}

/// Atomic renames using `renameat2`
///
/// # Errors
/// From the underlying filesystem syscall.
pub fn atomic_rename_blocking(old_path: PathBuf, new_path: PathBuf) -> io::Result<()> {
    if renameat2(
        AT_FDCWD,
        &old_path,
        AT_FDCWD,
        &new_path,
        RenameFlags::RENAME_NOREPLACE,
    )
    .is_err()
    {
        renameat2(
            AT_FDCWD,
            &old_path,
            AT_FDCWD,
            &new_path,
            RenameFlags::RENAME_EXCHANGE,
        )?;
    }

    Ok(())
}
