//! Extended attribute operations for POSIX compatibility.
//!
//! This module provides the [`FsXattr`] trait which enables extended
//! attribute (xattr) operations. Extended attributes are name-value pairs
//! associated with files and directories.
//!
//! # Overview
//!
//! Extended attributes provide a way to store metadata beyond the standard
//! file attributes (permissions, timestamps, etc.). Common uses include:
//!
//! - Security labels (SELinux, AppArmor)
//! - Access control lists (ACLs)
//! - Capabilities
//! - Custom application metadata
//!
//! # Example
//!
//! ```rust
//! use anyfs_backend::{FsXattr, FsError};
//! use std::path::Path;
//!
//! // Generic function that works with any FsXattr implementation
//! fn tag_file<B: FsXattr>(backend: &B, path: &Path, tag: &str) -> Result<(), FsError> {
//!     backend.set_xattr(path, "user.tag", tag.as_bytes())?;
//!     
//!     let tags = backend.list_xattr(path)?;
//!     println!("File has {} extended attributes", tags.len());
//!     Ok(())
//! }
//! ```
//!
//! # Namespaces
//!
//! Extended attribute names typically follow a namespace convention:
//! - `user.*` - User-defined attributes
//! - `system.*` - System-defined attributes
//! - `security.*` - Security-related attributes
//! - `trusted.*` - Trusted attributes (require privileges)
//!
//! # Thread Safety
//!
//! Like all AnyFS traits, `FsXattr` requires `Send + Sync`. Implementations
//! must handle concurrent access appropriately.

use std::path::Path;

use crate::FsError;

/// Extended attribute operations for POSIX compatibility.
///
/// This trait provides access to extended attributes (xattrs), which are
/// name-value pairs associated with files and directories.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsXattr, FsError};
/// use std::path::Path;
///
/// // Generic function that works with any FsXattr implementation
/// fn get_user_tag<B: FsXattr>(backend: &B, path: &Path) -> Result<String, FsError> {
///     let value = backend.get_xattr(path, "user.tag")?;
///     Ok(String::from_utf8_lossy(&value).into_owned())
/// }
/// ```
pub trait FsXattr: Send + Sync {
    /// Get an extended attribute value.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path
    /// * `name` - The attribute name (e.g., "user.tag")
    ///
    /// # Returns
    ///
    /// The attribute value as bytes.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path doesn't exist
    /// - [`FsError::XattrNotFound`] if the attribute doesn't exist
    /// - [`FsError::PermissionDenied`] if access is denied
    fn get_xattr(&self, path: &Path, name: &str) -> Result<Vec<u8>, FsError>;

    /// Set an extended attribute value.
    ///
    /// Creates the attribute if it doesn't exist, or updates it if it does.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path
    /// * `name` - The attribute name (e.g., "user.tag")
    /// * `value` - The attribute value
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path doesn't exist
    /// - [`FsError::PermissionDenied`] if access is denied
    /// - [`FsError::NotSupported`] if xattrs are not supported
    fn set_xattr(&self, path: &Path, name: &str, value: &[u8]) -> Result<(), FsError>;

    /// Remove an extended attribute.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path
    /// * `name` - The attribute name to remove
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path doesn't exist
    /// - [`FsError::XattrNotFound`] if the attribute doesn't exist
    /// - [`FsError::PermissionDenied`] if access is denied
    fn remove_xattr(&self, path: &Path, name: &str) -> Result<(), FsError>;

    /// List all extended attribute names for a path.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path
    ///
    /// # Returns
    ///
    /// A list of attribute names.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path doesn't exist
    /// - [`FsError::PermissionDenied`] if access is denied
    fn list_xattr(&self, path: &Path) -> Result<Vec<String>, FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::RwLock;

    /// Mock implementation of FsXattr for testing
    struct MockXattrFs {
        // path -> (name -> value)
        xattrs: RwLock<HashMap<PathBuf, HashMap<String, Vec<u8>>>>,
        // Set of existing paths
        paths: RwLock<std::collections::HashSet<PathBuf>>,
    }

    impl MockXattrFs {
        fn new() -> Self {
            Self {
                xattrs: RwLock::new(HashMap::new()),
                paths: RwLock::new(std::collections::HashSet::new()),
            }
        }

        fn add_path(&self, path: &Path) {
            self.paths.write().unwrap().insert(path.to_path_buf());
            self.xattrs
                .write()
                .unwrap()
                .entry(path.to_path_buf())
                .or_default();
        }
    }

    impl FsXattr for MockXattrFs {
        fn get_xattr(&self, path: &Path, name: &str) -> Result<Vec<u8>, FsError> {
            if !self.paths.read().unwrap().contains(path) {
                return Err(FsError::NotFound {
                    path: path.to_path_buf(),
                });
            }

            self.xattrs
                .read()
                .unwrap()
                .get(path)
                .and_then(|attrs| attrs.get(name).cloned())
                .ok_or_else(|| FsError::XattrNotFound {
                    path: path.to_path_buf(),
                    name: name.to_string(),
                })
        }

        fn set_xattr(&self, path: &Path, name: &str, value: &[u8]) -> Result<(), FsError> {
            if !self.paths.read().unwrap().contains(path) {
                return Err(FsError::NotFound {
                    path: path.to_path_buf(),
                });
            }

            self.xattrs
                .write()
                .unwrap()
                .entry(path.to_path_buf())
                .or_default()
                .insert(name.to_string(), value.to_vec());
            Ok(())
        }

        fn remove_xattr(&self, path: &Path, name: &str) -> Result<(), FsError> {
            if !self.paths.read().unwrap().contains(path) {
                return Err(FsError::NotFound {
                    path: path.to_path_buf(),
                });
            }

            let mut xattrs = self.xattrs.write().unwrap();
            if let Some(attrs) = xattrs.get_mut(path) {
                if attrs.remove(name).is_some() {
                    return Ok(());
                }
            }

            Err(FsError::XattrNotFound {
                path: path.to_path_buf(),
                name: name.to_string(),
            })
        }

        fn list_xattr(&self, path: &Path) -> Result<Vec<String>, FsError> {
            if !self.paths.read().unwrap().contains(path) {
                return Err(FsError::NotFound {
                    path: path.to_path_buf(),
                });
            }

            Ok(self
                .xattrs
                .read()
                .unwrap()
                .get(path)
                .map(|attrs| attrs.keys().cloned().collect())
                .unwrap_or_default())
        }
    }

    #[test]
    fn set_and_get_xattr() {
        let fs = MockXattrFs::new();
        fs.add_path(Path::new("/file.txt"));

        fs.set_xattr(Path::new("/file.txt"), "user.tag", b"test")
            .unwrap();
        let value = fs.get_xattr(Path::new("/file.txt"), "user.tag").unwrap();
        assert_eq!(value, b"test");
    }

    #[test]
    fn get_xattr_not_found() {
        let fs = MockXattrFs::new();
        fs.add_path(Path::new("/file.txt"));

        let result = fs.get_xattr(Path::new("/file.txt"), "user.missing");
        assert!(matches!(result, Err(FsError::XattrNotFound { .. })));
    }

    #[test]
    fn get_xattr_path_not_found() {
        let fs = MockXattrFs::new();
        let result = fs.get_xattr(Path::new("/missing.txt"), "user.tag");
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn set_xattr_path_not_found() {
        let fs = MockXattrFs::new();
        let result = fs.set_xattr(Path::new("/missing.txt"), "user.tag", b"value");
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn remove_xattr_succeeds() {
        let fs = MockXattrFs::new();
        fs.add_path(Path::new("/file.txt"));

        fs.set_xattr(Path::new("/file.txt"), "user.tag", b"test")
            .unwrap();
        fs.remove_xattr(Path::new("/file.txt"), "user.tag").unwrap();

        let result = fs.get_xattr(Path::new("/file.txt"), "user.tag");
        assert!(matches!(result, Err(FsError::XattrNotFound { .. })));
    }

    #[test]
    fn remove_xattr_not_found() {
        let fs = MockXattrFs::new();
        fs.add_path(Path::new("/file.txt"));

        let result = fs.remove_xattr(Path::new("/file.txt"), "user.missing");
        assert!(matches!(result, Err(FsError::XattrNotFound { .. })));
    }

    #[test]
    fn list_xattr_returns_names() {
        let fs = MockXattrFs::new();
        fs.add_path(Path::new("/file.txt"));

        fs.set_xattr(Path::new("/file.txt"), "user.tag", b"test")
            .unwrap();
        fs.set_xattr(Path::new("/file.txt"), "user.author", b"alice")
            .unwrap();

        let mut names = fs.list_xattr(Path::new("/file.txt")).unwrap();
        names.sort();
        assert_eq!(names, vec!["user.author", "user.tag"]);
    }

    #[test]
    fn list_xattr_empty() {
        let fs = MockXattrFs::new();
        fs.add_path(Path::new("/file.txt"));

        let names = fs.list_xattr(Path::new("/file.txt")).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn list_xattr_path_not_found() {
        let fs = MockXattrFs::new();
        let result = fs.list_xattr(Path::new("/missing.txt"));
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }
}
