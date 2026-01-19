//! Directory operations for virtual filesystems.

use std::path::Path;

use crate::{DirEntry, FsError};

/// Directory operations for a virtual filesystem.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync`. Methods use `&self` to allow
/// concurrent access.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `dyn FsDir`.
pub trait FsDir: Send + Sync {
    /// List directory contents.
    ///
    /// Returns an iterator over directory entries. The outer `Result` indicates
    /// whether the directory could be opened; each item's `Result` indicates
    /// whether that specific entry could be read.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotADirectory`] if the path is not a directory
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError>;

    /// Create a directory (parent must exist).
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if parent directory does not exist
    /// - [`FsError::AlreadyExists`] if the path already exists
    fn create_dir(&self, path: &Path) -> Result<(), FsError>;

    /// Create a directory and all parent directories.
    ///
    /// This is idempotent - succeeds if the directory already exists.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotADirectory`] if a component of the path exists but is not a directory
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError>;

    /// Remove an empty directory.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotADirectory`] if the path is not a directory
    /// - [`FsError::DirectoryNotEmpty`] if the directory is not empty
    fn remove_dir(&self, path: &Path) -> Result<(), FsError>;

    /// Remove a directory and all its contents recursively.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotADirectory`] if the path is not a directory
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError>;
}

/// Iterator over directory entries.
///
/// Wraps a boxed iterator for flexibility across different backends.
///
/// - Outer `Result` (from [`FsDir::read_dir`]) = "can I open this directory?"
/// - Inner `Result` (per item) = "can I read this entry?"
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{Fs, FsError};
/// use std::path::Path;
///
/// // Generic function that works with any Fs implementation
/// fn list_files<B: Fs>(backend: &B) -> Result<Vec<String>, FsError> {
///     let mut names = Vec::new();
///     for entry in backend.read_dir(Path::new("/"))? {
///         let entry = entry?;
///         names.push(entry.name);
///     }
///     Ok(names)
/// }
/// ```
pub struct ReadDirIter(Box<dyn Iterator<Item = Result<DirEntry, FsError>> + Send + 'static>);

impl ReadDirIter {
    /// Create from any compatible iterator.
    pub fn new<I>(iter: I) -> Self
    where
        I: Iterator<Item = Result<DirEntry, FsError>> + Send + 'static,
    {
        Self(Box::new(iter))
    }

    /// Create from a pre-collected vector.
    ///
    /// Useful for middleware like Overlay that merges multiple directory listings.
    pub fn from_vec(entries: Vec<Result<DirEntry, FsError>>) -> Self {
        Self(Box::new(entries.into_iter()))
    }

    /// Collect all entries, short-circuiting on first error.
    ///
    /// This is a convenience method equivalent to `iter.collect::<Result<Vec<_>, _>>()`.
    pub fn collect_all(self) -> Result<Vec<DirEntry>, FsError> {
        self.collect()
    }
}

impl Iterator for ReadDirIter {
    type Item = Result<DirEntry, FsError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileType;
    use std::path::PathBuf;

    #[test]
    fn read_dir_iter_from_vec() {
        let entries = vec![
            Ok(DirEntry {
                name: "a".into(),
                path: PathBuf::from("/a"),
                file_type: FileType::File,
                size: 0,
                inode: 1,
            }),
            Ok(DirEntry {
                name: "b".into(),
                path: PathBuf::from("/b"),
                file_type: FileType::Directory,
                size: 0,
                inode: 2,
            }),
        ];
        let iter = ReadDirIter::from_vec(entries);
        let collected: Vec<_> = iter.collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn read_dir_iter_collect_all_success() {
        let entries = vec![Ok(DirEntry {
            name: "a".into(),
            path: PathBuf::from("/a"),
            file_type: FileType::File,
            size: 100,
            inode: 1,
        })];
        let iter = ReadDirIter::from_vec(entries);
        let result = iter.collect_all();
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "a");
    }

    #[test]
    fn read_dir_iter_collect_all_error() {
        let entries: Vec<Result<DirEntry, FsError>> = vec![
            Ok(DirEntry {
                name: "a".into(),
                path: PathBuf::from("/a"),
                file_type: FileType::File,
                size: 0,
                inode: 1,
            }),
            Err(FsError::PermissionDenied {
                path: PathBuf::from("/b"),
                operation: "read_dir",
            }),
        ];
        let iter = ReadDirIter::from_vec(entries);
        let result = iter.collect_all();
        assert!(result.is_err());
    }

    #[test]
    fn read_dir_iter_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ReadDirIter>();
    }
}
