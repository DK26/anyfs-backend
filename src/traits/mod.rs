//! Filesystem traits defining the AnyFS interface hierarchy.
//!
//! ## Trait Layers
//!
//! - **Layer 1 (Core):** [`FsRead`], [`FsWrite`], [`FsDir`] → [`Fs`]
//! - **Layer 2 (Extended):** `FsLink`, `FsPermissions`, `FsSync`, `FsStats` → `FsFull`
//! - **Layer 3 (FUSE):** `FsInode` → `FsFuse`
//! - **Layer 4 (POSIX):** `FsHandles`, `FsLock`, `FsXattr` → `FsPosix`

mod fs_dir;
mod fs_read;
mod fs_write;

// Layer 1 - Core traits
pub use fs_dir::{FsDir, ReadDirIter};
pub use fs_read::FsRead;
pub use fs_write::FsWrite;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_trait_is_object_safe() {
        // This test verifies that Fs can be used as a trait object
        fn _check(_: &dyn Fs) {}
    }
}
