//! # Extension Traits
//!
//! Convenience methods for filesystem backends.
//!
//! ## Overview
//!
//! [`FsExt`] provides commonly-needed utility methods that aren't part of
//! the core trait hierarchy. These are implemented as default methods with
//! blanket implementations, so any `Fs` backend gets them for free.
//!
//! ## Available Methods
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`is_file`](FsExt::is_file) | Check if path is a regular file |
//! | [`is_dir`](FsExt::is_dir) | Check if path is a directory |
//!
//! ## JSON Support (Feature-Gated)
//!
//! With the `serde` feature enabled, additional methods are available:
//!
//! | Method | Description |
//! |--------|-------------|
//! | `read_json` | Read and deserialize JSON file |
//! | `write_json` | Serialize and write JSON file |
//!
//! Enable with:
//! ```toml
//! [dependencies]
//! anyfs-backend = { version = "0.1", features = ["serde"] }
//! ```

use crate::{Fs, FsError};
use std::path::Path;

/// Extension methods for any filesystem backend.
///
/// Provides convenience methods not in the core traits but commonly needed.
/// All methods have default implementations, so backends get them automatically.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{Fs, FsExt, FsError};
/// use std::path::Path;
///
/// fn check_paths<B: Fs>(backend: &B) -> Result<(), FsError> {
///     // FsExt methods are available on any Fs backend
///     if backend.is_file(Path::new("/config.json"))? {
///         println!("Config exists!");
///     }
///     
///     if backend.is_dir(Path::new("/data"))? {
///         println!("Data directory exists!");
///     }
///     
///     Ok(())
/// }
/// ```
pub trait FsExt: Fs {
    /// Check if the path points to a regular file.
    ///
    /// Returns `Ok(false)` if the path doesn't exist (not an error).
    /// Returns `Err` only for actual I/O errors (permission denied, etc.).
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyfs_backend::{Fs, FsExt, FsError};
    /// use std::path::Path;
    ///
    /// fn process_file<B: Fs>(backend: &B, path: &Path) -> Result<(), FsError> {
    ///     if backend.is_file(path)? {
    ///         let data = backend.read(path)?;
    ///         // Process the file...
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn is_file(&self, path: &Path) -> Result<bool, FsError> {
        match self.metadata(path) {
            Ok(m) => Ok(m.is_file()),
            Err(FsError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Check if the path points to a directory.
    ///
    /// Returns `Ok(false)` if the path doesn't exist (not an error).
    /// Returns `Err` only for actual I/O errors (permission denied, etc.).
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyfs_backend::{Fs, FsExt, FsError};
    /// use std::path::Path;
    ///
    /// fn ensure_dir<B: Fs>(backend: &B, path: &Path) -> Result<(), FsError> {
    ///     if !backend.is_dir(path)? {
    ///         backend.create_dir_all(path)?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn is_dir(&self, path: &Path) -> Result<bool, FsError> {
        match self.metadata(path) {
            Ok(m) => Ok(m.is_dir()),
            Err(FsError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Check if the path points to a symbolic link.
    ///
    /// Returns `Ok(false)` if the path doesn't exist (not an error).
    /// Returns `Err` only for actual I/O errors.
    ///
    /// # Note
    ///
    /// This method uses regular `metadata()` which follows symlinks.
    /// For backends implementing [`FsLink`](crate::FsLink), consider using
    /// `symlink_metadata()` for more accurate symlink detection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyfs_backend::{Fs, FsExt, FsError};
    /// use std::path::Path;
    ///
    /// fn check_link<B: Fs>(backend: &B, path: &Path) -> Result<bool, FsError> {
    ///     backend.is_symlink(path)
    /// }
    /// ```
    fn is_symlink(&self, path: &Path) -> Result<bool, FsError> {
        match self.metadata(path) {
            Ok(m) => Ok(m.is_symlink()),
            Err(FsError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get the size of a file in bytes.
    ///
    /// Convenience method that extracts just the size from metadata.
    ///
    /// # Errors
    ///
    /// Returns `FsError::NotFound` if the path doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyfs_backend::{Fs, FsExt, FsError};
    /// use std::path::Path;
    ///
    /// fn check_size<B: Fs>(backend: &B, path: &Path) -> Result<u64, FsError> {
    ///     backend.file_size(path)
    /// }
    /// ```
    fn file_size(&self, path: &Path) -> Result<u64, FsError> {
        Ok(self.metadata(path)?.size)
    }
}

// Blanket implementation - any Fs backend gets FsExt for free
impl<B: Fs + ?Sized> FsExt for B {}

// =============================================================================
// JSON Support (Feature-Gated)
// =============================================================================

#[cfg(feature = "serde")]
mod json {
    use super::*;
    use serde::{de::DeserializeOwned, Serialize};

    /// JSON serialization extension methods.
    ///
    /// Available when the `serde` feature is enabled.
    pub trait FsExtJson: Fs {
        /// Read a file and deserialize it as JSON.
        ///
        /// # Type Parameters
        ///
        /// - `T`: The type to deserialize into (must implement `DeserializeOwned`)
        ///
        /// # Errors
        ///
        /// - `FsError::NotFound` — File doesn't exist
        /// - `FsError::InvalidData` — File isn't valid UTF-8
        /// - `FsError::Deserialization` — JSON parsing failed
        ///
        /// # Example
        ///
        /// ```rust
        /// use anyfs_backend::{Fs, FsError};
        /// #[cfg(feature = "serde")]
        /// use anyfs_backend::FsExtJson;
        /// use std::path::Path;
        ///
        /// #[cfg(feature = "serde")]
        /// fn load_config<B: Fs>(backend: &B) -> Result<serde_json::Value, FsError> {
        ///     backend.read_json(Path::new("/config.json"))
        /// }
        /// ```
        fn read_json<T: DeserializeOwned>(&self, path: &Path) -> Result<T, FsError> {
            let data = self.read_to_string(path)?;
            serde_json::from_str(&data).map_err(|e| FsError::Deserialization(e.to_string()))
        }

        /// Serialize a value and write it as JSON.
        ///
        /// Uses pretty-printing with 2-space indentation.
        ///
        /// # Type Parameters
        ///
        /// - `T`: The type to serialize (must implement `Serialize`)
        ///
        /// # Errors
        ///
        /// - `FsError::Serialization` — JSON serialization failed
        /// - Other `FsError` variants from the underlying `write()` call
        ///
        /// # Example
        ///
        /// ```rust
        /// use anyfs_backend::{Fs, FsError};
        /// #[cfg(feature = "serde")]
        /// use anyfs_backend::FsExtJson;
        /// use std::path::Path;
        ///
        /// #[cfg(feature = "serde")]
        /// fn save_value<B: Fs>(backend: &B, value: &serde_json::Value) -> Result<(), FsError> {
        ///     backend.write_json(Path::new("/config.json"), value)
        /// }
        /// ```
        fn write_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), FsError> {
            let json = serde_json::to_string_pretty(value)
                .map_err(|e| FsError::Serialization(e.to_string()))?;
            self.write(path, json.as_bytes())
        }
    }

    // Blanket implementation
    impl<B: Fs + ?Sized> FsExtJson for B {}
}

#[cfg(feature = "serde")]
pub use json::FsExtJson;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FsDir, FsRead, FsWrite, Metadata, ReadDirIter};
    use std::io::{Read, Write};

    /// Mock backend for testing
    struct MockFs {
        file_exists: bool,
        dir_exists: bool,
    }

    impl MockFs {
        fn with_file() -> Self {
            Self {
                file_exists: true,
                dir_exists: false,
            }
        }

        fn with_dir() -> Self {
            Self {
                file_exists: false,
                dir_exists: true,
            }
        }

        fn empty() -> Self {
            Self {
                file_exists: false,
                dir_exists: false,
            }
        }
    }

    impl FsRead for MockFs {
        fn read(&self, _: &Path) -> Result<Vec<u8>, FsError> {
            Ok(vec![])
        }

        fn read_to_string(&self, _: &Path) -> Result<String, FsError> {
            Ok(String::new())
        }

        fn read_range(&self, _: &Path, _: u64, _: usize) -> Result<Vec<u8>, FsError> {
            Ok(vec![])
        }

        fn exists(&self, _: &Path) -> Result<bool, FsError> {
            Ok(self.file_exists || self.dir_exists)
        }

        fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
            if self.file_exists {
                Ok(Metadata {
                    file_type: crate::FileType::File,
                    size: 100,
                    ..Metadata::default()
                })
            } else if self.dir_exists {
                Ok(Metadata {
                    file_type: crate::FileType::Directory,
                    size: 0,
                    ..Metadata::default()
                })
            } else {
                Err(FsError::NotFound {
                    path: path.to_path_buf(),
                })
            }
        }

        fn open_read(&self, _: &Path) -> Result<Box<dyn Read + Send>, FsError> {
            Ok(Box::new(std::io::empty()))
        }
    }

    impl FsWrite for MockFs {
        fn write(&self, _: &Path, _: &[u8]) -> Result<(), FsError> {
            Ok(())
        }

        fn append(&self, _: &Path, _: &[u8]) -> Result<(), FsError> {
            Ok(())
        }

        fn truncate(&self, _: &Path, _: u64) -> Result<(), FsError> {
            Ok(())
        }

        fn remove_file(&self, _: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn rename(&self, _: &Path, _: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn copy(&self, _: &Path, _: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn open_write(&self, _: &Path) -> Result<Box<dyn Write + Send>, FsError> {
            Ok(Box::new(std::io::sink()))
        }
    }

    impl FsDir for MockFs {
        fn read_dir(&self, _: &Path) -> Result<ReadDirIter, FsError> {
            Ok(ReadDirIter::from_vec(vec![]))
        }

        fn create_dir(&self, _: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn create_dir_all(&self, _: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn remove_dir(&self, _: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn remove_dir_all(&self, _: &Path) -> Result<(), FsError> {
            Ok(())
        }
    }

    #[test]
    fn is_file_returns_true_for_files() {
        let fs = MockFs::with_file();
        assert!(fs.is_file(Path::new("/test.txt")).unwrap());
    }

    #[test]
    fn is_file_returns_false_for_dirs() {
        let fs = MockFs::with_dir();
        assert!(!fs.is_file(Path::new("/dir")).unwrap());
    }

    #[test]
    fn is_file_returns_false_for_missing() {
        let fs = MockFs::empty();
        assert!(!fs.is_file(Path::new("/missing")).unwrap());
    }

    #[test]
    fn is_dir_returns_true_for_dirs() {
        let fs = MockFs::with_dir();
        assert!(fs.is_dir(Path::new("/dir")).unwrap());
    }

    #[test]
    fn is_dir_returns_false_for_files() {
        let fs = MockFs::with_file();
        assert!(!fs.is_dir(Path::new("/test.txt")).unwrap());
    }

    #[test]
    fn is_dir_returns_false_for_missing() {
        let fs = MockFs::empty();
        assert!(!fs.is_dir(Path::new("/missing")).unwrap());
    }

    #[test]
    fn file_size_returns_size() {
        let fs = MockFs::with_file();
        assert_eq!(fs.file_size(Path::new("/test.txt")).unwrap(), 100);
    }

    #[test]
    fn file_size_errors_on_missing() {
        let fs = MockFs::empty();
        let result = fs.file_size(Path::new("/missing"));
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn fs_ext_available_on_dyn_fs() {
        let fs: &dyn Fs = &MockFs::with_file();
        // FsExt methods work on trait objects
        assert!(fs.is_file(Path::new("/test.txt")).unwrap());
    }
}
