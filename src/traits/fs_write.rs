//! Write operations for virtual filesystems.

use std::io::Write;
use std::path::Path;

use crate::FsError;

/// Write operations for a virtual filesystem.
///
/// All methods use `&self` (interior mutability). Backends manage their own synchronization.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync`. Methods use `&self` to allow
/// concurrent access. Backends should use interior mutability (`RwLock`, `Mutex`)
/// for thread-safe state management.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `dyn FsWrite`.
pub trait FsWrite: Send + Sync {
    /// Write data to a file (creates if not exists, truncates if exists).
    ///
    /// Parent directories must exist. Use [`FsDir::create_dir_all`](super::FsDir::create_dir_all)
    /// to ensure parent directories exist.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if parent directory does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    /// - [`FsError::PermissionDenied`] if write access is denied
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;

    /// Append data to a file (creates if not exists).
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if parent directory does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;

    /// Remove a file.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the file does not exist
    /// - [`FsError::NotAFile`] if the path is a directory (use [`FsDir::remove_dir`](super::FsDir::remove_dir))
    fn remove_file(&self, path: &Path) -> Result<(), FsError>;

    /// Rename/move a file or directory.
    ///
    /// This operation should be atomic where possible.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the source path does not exist
    /// - [`FsError::AlreadyExists`] if the destination already exists (backend-specific)
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// Copy a file.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the source file does not exist
    /// - [`FsError::NotAFile`] if the source is a directory
    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// Truncate a file to the specified size.
    ///
    /// If the file is larger than `size`, the extra data is discarded.
    /// If the file is smaller, it is extended with zero bytes.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the file does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError>;

    /// Open a file for writing, returning a boxed writer.
    ///
    /// This is a "cold path" operation that returns a trait object for flexibility.
    /// For hot path writes, prefer [`write`](Self::write).
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if parent directory does not exist
    /// - [`FsError::NotAFile`] if the path is a directory
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError>;
}
