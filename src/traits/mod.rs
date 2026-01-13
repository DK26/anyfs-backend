//! Filesystem traits defining the AnyFS interface hierarchy.
//!
//! ## Trait Layers
//!
//! - **Layer 1 (Core):** [`FsRead`], [`FsWrite`], [`FsDir`] → [`Fs`]
//! - **Layer 2 (Extended):** [`FsLink`], [`FsPermissions`], [`FsSync`], [`FsStats`] → [`FsFull`]
//! - **Layer 3 (FUSE):** `FsInode` → `FsFuse`
//! - **Layer 4 (POSIX):** `FsHandles`, `FsLock`, `FsXattr` → `FsPosix`

mod fs_dir;
mod fs_link;
mod fs_path;
mod fs_permissions;
mod fs_read;
mod fs_stats;
mod fs_sync;
mod fs_write;

// Layer 1 - Core traits
pub use fs_dir::{FsDir, ReadDirIter};
pub use fs_read::FsRead;
pub use fs_write::FsWrite;

// Layer 2 - Extended traits
pub use fs_link::FsLink;
pub use fs_path::FsPath;
pub use fs_permissions::FsPermissions;
pub use fs_stats::FsStats;
pub use fs_sync::FsSync;

/// Basic filesystem - covers 90% of use cases.
///
/// This is the primary trait most users will depend on. It provides all basic
/// file and directory operations needed for typical filesystem interactions.
///
/// # Example
///
/// ```rust,ignore
/// use anyfs_backend::{Fs, FsError};
/// use std::path::Path;
///
/// fn process_files<B: Fs>(backend: &B) -> Result<(), FsError> {
///     let data = backend.read(Path::new("/input.txt"))?;
///     backend.write(Path::new("/output.txt"), &data)?;
///     Ok(())
/// }
/// ```
pub trait Fs: FsRead + FsWrite + FsDir {}

// Blanket implementation - any type implementing all three gets Fs for free
impl<T: FsRead + FsWrite + FsDir> Fs for T {}

/// Full filesystem with all std::fs features.
///
/// Includes basic operations ([`Fs`]) plus links, permissions, sync, and stats.
///
/// # Example
///
/// ```rust,ignore
/// use anyfs_backend::{FsFull, FsError};
/// use std::path::Path;
///
/// fn backup_with_links<B: FsFull>(backend: &B) -> Result<(), FsError> {
///     backend.write(Path::new("/file.txt"), b"data")?;
///     backend.hard_link(Path::new("/file.txt"), Path::new("/backup.txt"))?;
///     backend.sync()?;
///     Ok(())
/// }
/// ```
pub trait FsFull: Fs + FsLink + FsPermissions + FsSync + FsStats {}

// Blanket implementation
impl<T: Fs + FsLink + FsPermissions + FsSync + FsStats> FsFull for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_trait_is_object_safe() {
        // This test verifies that Fs can be used as a trait object
        fn _check(_: &dyn Fs) {}
    }

    #[test]
    fn fs_full_trait_is_object_safe() {
        // This test verifies that FsFull can be used as a trait object
        fn _check(_: &dyn FsFull) {}
    }
}
