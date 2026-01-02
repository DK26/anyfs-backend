//! Filesystem synchronization operations.

use std::path::Path;

use crate::FsError;

/// Filesystem synchronization operations.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync`. Methods use `&self` to allow
/// concurrent access.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `dyn FsSync`.
pub trait FsSync: Send + Sync {
    /// Sync all pending changes to persistent storage.
    ///
    /// This is a global sync that flushes all pending writes.
    ///
    /// # Errors
    ///
    /// - [`FsError::Io`] for underlying I/O errors
    fn sync(&self) -> Result<(), FsError>;

    /// Sync a specific file's data and metadata to storage.
    ///
    /// Similar to POSIX `fsync(fd)`.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::Io`] for underlying I/O errors
    fn fsync(&self, path: &Path) -> Result<(), FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_sync_is_object_safe() {
        fn _check(_: &dyn FsSync) {}
    }

    #[test]
    fn fs_sync_requires_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        fn _check<T: FsSync>() {
            _assert_send_sync::<T>();
        }
    }
}
