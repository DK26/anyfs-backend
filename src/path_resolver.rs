//! # PathResolver Trait
//!
//! Strategy trait for pluggable path resolution algorithms.
//!
//! ## Responsibility
//! - Define the contract for path resolution (canonicalization, normalization)
//!
//! ## Dependencies
//! - [`Fs`] trait for filesystem queries
//! - [`FsError`] for error handling
//!
//! ## Usage
//!
//! ```rust,ignore
//! use anyfs_backend::{PathResolver, Fs, FsError};
//! use std::path::{Path, PathBuf};
//!
//! struct MyCustomResolver;
//!
//! impl PathResolver for MyCustomResolver {
//!     fn canonicalize(&self, path: &Path, fs: &dyn Fs) -> Result<PathBuf, FsError> {
//!         // Custom resolution logic
//!         todo!()
//!     }
//!     
//!     fn soft_canonicalize(&self, path: &Path, fs: &dyn Fs) -> Result<PathBuf, FsError> {
//!         // Custom resolution logic (allows non-existent final component)
//!         todo!()
//!     }
//! }
//! ```

use std::path::{Path, PathBuf};

use crate::{Fs, FsError};

// ============================================================================
// Trait Definition
// ============================================================================

/// Strategy trait for path resolution algorithms.
///
/// Encapsulates how paths are normalized, symlinks are followed,
/// and `..`/`.` components are resolved.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support concurrent access.
///
/// # Object Safety
///
/// Uses `&dyn Fs` to remain object-safe, enabling runtime resolver selection.
///
/// # Symlink Handling
///
/// The trait accepts `&dyn Fs` for object safety. Implementations that need
/// symlink awareness can attempt to downcast to check for `FsLink` capabilities.
/// All built-in virtual backends implement `FsLink`, so symlink-aware resolution
/// works out of the box. For backends without `FsLink`, resolution still works
/// but treats all entries as non-symlinks.
///
/// # Implementors
///
/// - `IterativeResolver` (default in `anyfs`): Walks path component by component
/// - `NoOpResolver` (in `anyfs`): Pass-through for `SelfResolving` backends
/// - `CachingResolver` (in `anyfs`): LRU cache wrapper for any resolver (with TTL expiration)
///
/// # Example
///
/// ```rust,ignore
/// use anyfs_backend::{PathResolver, Fs, FsError};
/// use std::path::{Path, PathBuf};
///
/// struct MyCustomResolver;
///
/// impl PathResolver for MyCustomResolver {
///     fn canonicalize(&self, path: &Path, fs: &dyn Fs) -> Result<PathBuf, FsError> {
///         // Custom resolution logic
///         todo!()
///     }
///     
///     fn soft_canonicalize(&self, path: &Path, fs: &dyn Fs) -> Result<PathBuf, FsError> {
///         // Custom resolution logic (allows non-existent final component)
///         todo!()
///     }
/// }
/// ```
pub trait PathResolver: Send + Sync {
    /// Resolve path to canonical form.
    ///
    /// All symlinks are resolved, `.` and `..` are normalized,
    /// and all path components must exist.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to canonicalize
    /// * `fs` - The filesystem to query for path resolution
    ///
    /// # Returns
    ///
    /// The fully resolved canonical path.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] - A component doesn't exist
    /// - [`FsError::InvalidData`] - Symlink loop detected (circular symlinks)
    fn canonicalize(&self, path: &Path, fs: &dyn Fs) -> Result<PathBuf, FsError>;

    /// Like [`canonicalize`](Self::canonicalize), but allows non-existent final component.
    ///
    /// Resolves parent path fully, appends final component lexically.
    /// This is useful for `write()` operations where the target file
    /// doesn't exist yet.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to soft-canonicalize
    /// * `fs` - The filesystem to query for path resolution
    ///
    /// # Returns
    ///
    /// The resolved path with the final component appended lexically.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] - A parent component doesn't exist
    /// - [`FsError::InvalidData`] - Symlink loop detected
    fn soft_canonicalize(&self, path: &Path, fs: &dyn Fs) -> Result<PathBuf, FsError>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileType, FsDir, FsRead, FsWrite, Metadata, Permissions, ReadDirIter};
    use std::io::{Read, Write};
    use std::time::SystemTime;

    // Mock filesystem for testing
    struct MockFs;

    impl FsRead for MockFs {
        fn read(&self, _path: &Path) -> Result<Vec<u8>, FsError> {
            Ok(vec![])
        }

        fn read_to_string(&self, _path: &Path) -> Result<String, FsError> {
            Ok(String::new())
        }

        fn read_range(&self, _path: &Path, _offset: u64, _len: usize) -> Result<Vec<u8>, FsError> {
            Ok(vec![])
        }

        fn exists(&self, _path: &Path) -> Result<bool, FsError> {
            Ok(true)
        }

        fn metadata(&self, _path: &Path) -> Result<Metadata, FsError> {
            Ok(Metadata {
                file_type: FileType::File,
                size: 0,
                permissions: Permissions::default_file(),
                created: SystemTime::UNIX_EPOCH,
                modified: SystemTime::UNIX_EPOCH,
                accessed: SystemTime::UNIX_EPOCH,
                inode: 1,
                nlink: 1,
            })
        }

        fn open_read(&self, _path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
            Ok(Box::new(std::io::empty()))
        }
    }

    impl FsWrite for MockFs {
        fn write(&self, _path: &Path, _data: &[u8]) -> Result<(), FsError> {
            Ok(())
        }

        fn append(&self, _path: &Path, _data: &[u8]) -> Result<(), FsError> {
            Ok(())
        }

        fn remove_file(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn rename(&self, _from: &Path, _to: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn copy(&self, _from: &Path, _to: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn truncate(&self, _path: &Path, _size: u64) -> Result<(), FsError> {
            Ok(())
        }

        fn open_write(&self, _path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
            Ok(Box::new(std::io::sink()))
        }
    }

    impl FsDir for MockFs {
        fn read_dir(&self, _path: &Path) -> Result<ReadDirIter, FsError> {
            Ok(ReadDirIter::from_vec(vec![]))
        }

        fn create_dir(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn create_dir_all(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn remove_dir(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn remove_dir_all(&self, _path: &Path) -> Result<(), FsError> {
            Ok(())
        }
    }

    // Simple pass-through resolver for testing
    struct TestResolver;

    impl PathResolver for TestResolver {
        fn canonicalize(&self, path: &Path, _fs: &dyn Fs) -> Result<PathBuf, FsError> {
            // Simple: just return the path as-is (no actual resolution)
            Ok(path.to_path_buf())
        }

        fn soft_canonicalize(&self, path: &Path, _fs: &dyn Fs) -> Result<PathBuf, FsError> {
            Ok(path.to_path_buf())
        }
    }

    #[test]
    fn path_resolver_can_be_boxed() {
        // Verify PathResolver can be boxed for dynamic dispatch
        let resolver: Box<dyn PathResolver> = Box::new(TestResolver);
        let mock_fs = MockFs;
        let path = Path::new("/test/path");

        let result = resolver.canonicalize(path, &mock_fs);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/test/path"));
    }

    #[test]
    fn path_resolver_canonicalize_returns_path() {
        let resolver = TestResolver;
        let mock_fs = MockFs;
        let path = Path::new("/some/file.txt");

        let result = resolver.canonicalize(path, &mock_fs);
        assert!(result.is_ok());
    }

    #[test]
    fn path_resolver_soft_canonicalize_returns_path() {
        let resolver = TestResolver;
        let mock_fs = MockFs;
        let path = Path::new("/some/new/file.txt");

        let result = resolver.soft_canonicalize(path, &mock_fs);
        assert!(result.is_ok());
    }
}
