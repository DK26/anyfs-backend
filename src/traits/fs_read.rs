//! Read operations for virtual filesystems.

use std::io::Read;
use std::path::Path;

use crate::{FsError, Metadata};

/// Read operations for a virtual filesystem.
///
/// All methods use `&self` (interior mutability). Backends manage their own synchronization.
/// See ADR-023 for rationale.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync`. Methods use `&self` to allow
/// concurrent access. Backends should use interior mutability (`RwLock`, `Mutex`)
/// for thread-safe state management.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `dyn FsRead`.
pub trait FsRead: Send + Sync {
    /// Read entire file contents as bytes.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    /// - [`FsError::PermissionDenied`] if read access is denied
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;

    /// Read file contents as UTF-8 string.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    /// - [`FsError::InvalidData`] if the file contains invalid UTF-8
    fn read_to_string(&self, path: &Path) -> Result<String, FsError>;

    /// Read a range of bytes from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `offset` - Byte offset to start reading from
    /// * `len` - Maximum number of bytes to read
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError>;

    /// Check if a path exists.
    ///
    /// Returns `Ok(true)` if the path exists, `Ok(false)` if it does not.
    /// Only returns an error for unexpected failures (e.g., I/O errors).
    fn exists(&self, path: &Path) -> Result<bool, FsError>;

    /// Get metadata for a path (follows symlinks).
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError>;

    /// Open a file for reading, returning a boxed reader.
    ///
    /// This is a "cold path" operation that returns a trait object for flexibility.
    /// For hot path reads, prefer [`read`](Self::read) or [`read_range`](Self::read_range).
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_read_is_object_safe() {
        // This test verifies that FsRead can be used as a trait object
        fn _check(_: &dyn FsRead) {}
    }

    #[test]
    fn fs_read_requires_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        // This would fail to compile if FsRead didn't require Send + Sync
        fn _check<T: FsRead>() {
            _assert_send_sync::<T>();
        }
    }
}
