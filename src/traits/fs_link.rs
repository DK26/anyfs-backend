//! Symlink and hard link operations.

use std::path::{Path, PathBuf};

use crate::{FsError, Metadata};

/// Symlink and hard link operations.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync`. Methods use `&self` to allow
/// concurrent access.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `dyn FsLink`.
pub trait FsLink: Send + Sync {
    /// Create a symbolic link.
    ///
    /// # Arguments
    ///
    /// * `target` - The path the symlink points to (does not need to exist)
    /// * `link` - The path where the symlink is created
    ///
    /// # Errors
    ///
    /// - [`FsError::AlreadyExists`] if `link` already exists
    /// - [`FsError::NotFound`] if parent of `link` does not exist
    fn symlink(&self, target: &Path, link: &Path) -> Result<(), FsError>;

    /// Create a hard link.
    ///
    /// # Arguments
    ///
    /// * `original` - The existing file to link to (must exist and be a file)
    /// * `link` - The path for the new hard link
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if `original` does not exist
    /// - [`FsError::NotAFile`] if `original` is a directory
    /// - [`FsError::AlreadyExists`] if `link` already exists
    fn hard_link(&self, original: &Path, link: &Path) -> Result<(), FsError>;

    /// Read the target of a symbolic link.
    ///
    /// Returns the raw target path (not canonicalized).
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if `path` does not exist
    /// - [`FsError::InvalidData`] if `path` is not a symlink
    fn read_link(&self, path: &Path) -> Result<PathBuf, FsError>;

    /// Get metadata without following symlinks.
    ///
    /// Unlike [`FsRead::metadata`](super::FsRead::metadata), this does not
    /// follow symlinks. If `path` is a symlink, returns the symlink's metadata.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if `path` does not exist
    fn symlink_metadata(&self, path: &Path) -> Result<Metadata, FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_link_is_object_safe() {
        fn _check(_: &dyn FsLink) {}
    }

    #[test]
    fn fs_link_requires_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        fn _check<T: FsLink>() {
            _assert_send_sync::<T>();
        }
    }
}
