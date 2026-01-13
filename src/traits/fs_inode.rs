//! Inode-based filesystem operations for FUSE mounting.
//!
//! This module provides the [`FsInode`] trait which enables inode-based
//! operations required for FUSE filesystem implementations.
//!
//! # Overview
//!
//! FUSE (Filesystem in Userspace) operates on inodes rather than paths for
//! efficiency. The `FsInode` trait provides the necessary mappings between
//! paths and inodes, as well as inode-based metadata lookup.
//!
//! # Example
//!
//! ```rust
//! use anyfs_backend::{FsInode, FsError, Metadata};
//! use std::path::Path;
//! use std::ffi::OsStr;
//!
//! // Generic function that works with any FsInode implementation
//! fn get_child_metadata<B: FsInode>(
//!     backend: &B,
//!     parent: u64,
//!     name: &OsStr,
//! ) -> Result<Metadata, FsError> {
//!     let child_inode = backend.lookup(parent, name)?;
//!     backend.metadata_by_inode(child_inode)
//! }
//! ```
//!
//! # Thread Safety
//!
//! Like all AnyFS traits, `FsInode` requires `Send + Sync`. Implementations
//! must use interior mutability for any mutable state.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::{FsError, Metadata};

/// Inode-based filesystem operations for FUSE mounting.
///
/// This trait provides the bridge between path-based and inode-based
/// operations. FUSE implementations require efficient inode lookups
/// and path-to-inode mappings.
///
/// # Root Inode
///
/// The root inode is conventionally `1` (see [`crate::ROOT_INODE`]).
/// Implementations should ensure that path "/" maps to inode 1.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsInode, FsError, Metadata, ROOT_INODE};
/// use std::path::{Path, PathBuf};
/// use std::ffi::OsStr;
///
/// struct MyInodeFs { /* ... */ }
///
/// impl FsInode for MyInodeFs {
///     fn path_to_inode(&self, path: &Path) -> Result<u64, FsError> {
///         // Map path to inode
///         if path == Path::new("/") {
///             Ok(ROOT_INODE)
///         } else {
///             // ... lookup in inode table
///             Ok(2)
///         }
///     }
///
///     fn inode_to_path(&self, inode: u64) -> Result<PathBuf, FsError> {
///         // Map inode back to path
///         if inode == ROOT_INODE {
///             Ok(PathBuf::from("/"))
///         } else {
///             // ... lookup in path table
///             Ok(PathBuf::from("/file.txt"))
///         }
///     }
///
///     fn lookup(&self, _parent_inode: u64, _name: &OsStr) -> Result<u64, FsError> {
///         // Find child inode by name within parent directory
///         Ok(2)
///     }
///
///     fn metadata_by_inode(&self, _inode: u64) -> Result<Metadata, FsError> {
///         // Get metadata directly by inode
///         Ok(Metadata::default())
///     }
/// }
/// ```
pub trait FsInode: Send + Sync {
    /// Convert a path to its inode number.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path to look up
    ///
    /// # Returns
    ///
    /// The inode number for the path.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::NotADirectory`] if a component is not a directory
    /// - [`FsError::PermissionDenied`] if access is denied
    fn path_to_inode(&self, path: &Path) -> Result<u64, FsError>;

    /// Convert an inode number back to its path.
    ///
    /// # Arguments
    ///
    /// * `inode` - The inode number to look up
    ///
    /// # Returns
    ///
    /// The filesystem path for the inode.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the inode does not exist
    ///
    /// # Note
    ///
    /// For filesystems with hard links, an inode may have multiple paths.
    /// This method returns one valid path (typically the canonical one).
    fn inode_to_path(&self, inode: u64) -> Result<PathBuf, FsError>;

    /// Look up a child entry within a parent directory by name.
    ///
    /// This is the core FUSE lookup operation. Given a parent directory's
    /// inode and a child name, return the child's inode.
    ///
    /// # Arguments
    ///
    /// * `parent_inode` - The inode of the parent directory
    /// * `name` - The name of the child entry to find
    ///
    /// # Returns
    ///
    /// The inode number of the child entry.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the child does not exist
    /// - [`FsError::NotADirectory`] if parent is not a directory
    /// - [`FsError::PermissionDenied`] if access is denied
    fn lookup(&self, parent_inode: u64, name: &OsStr) -> Result<u64, FsError>;

    /// Get metadata for an inode directly.
    ///
    /// This is more efficient than `inode_to_path` + `metadata` for FUSE
    /// operations, as it avoids path string manipulation.
    ///
    /// # Arguments
    ///
    /// * `inode` - The inode number to get metadata for
    ///
    /// # Returns
    ///
    /// The metadata for the inode.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the inode does not exist
    fn metadata_by_inode(&self, inode: u64) -> Result<Metadata, FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileType, ROOT_INODE};
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// Mock implementation of FsInode for testing
    struct MockInodeFs {
        // Maps inode -> (path, file_type)
        inodes: RwLock<HashMap<u64, (PathBuf, FileType)>>,
        // Maps path -> inode
        paths: RwLock<HashMap<PathBuf, u64>>,
        // Maps (parent_inode, name) -> child_inode
        children: RwLock<HashMap<(u64, String), u64>>,
    }

    impl MockInodeFs {
        fn new() -> Self {
            let mut inodes = HashMap::new();
            let mut paths = HashMap::new();

            // Root directory
            inodes.insert(ROOT_INODE, (PathBuf::from("/"), FileType::Directory));
            paths.insert(PathBuf::from("/"), ROOT_INODE);

            Self {
                inodes: RwLock::new(inodes),
                paths: RwLock::new(paths),
                children: RwLock::new(HashMap::new()),
            }
        }

        fn add_file(&self, path: &Path, inode: u64, file_type: FileType) {
            self.inodes
                .write()
                .unwrap()
                .insert(inode, (path.to_path_buf(), file_type));
            self.paths
                .write()
                .unwrap()
                .insert(path.to_path_buf(), inode);

            // Add to parent's children
            if let Some(parent) = path.parent() {
                if let Some(name) = path.file_name() {
                    let parent_inode = *self.paths.read().unwrap().get(parent).unwrap_or(&1);
                    self.children
                        .write()
                        .unwrap()
                        .insert((parent_inode, name.to_string_lossy().into_owned()), inode);
                }
            }
        }
    }

    impl FsInode for MockInodeFs {
        fn path_to_inode(&self, path: &Path) -> Result<u64, FsError> {
            self.paths
                .read()
                .unwrap()
                .get(path)
                .copied()
                .ok_or_else(|| FsError::NotFound {
                    path: path.to_path_buf(),
                })
        }

        fn inode_to_path(&self, inode: u64) -> Result<PathBuf, FsError> {
            self.inodes
                .read()
                .unwrap()
                .get(&inode)
                .map(|(path, _)| path.clone())
                .ok_or(FsError::InodeNotFound { inode })
        }

        fn lookup(&self, parent_inode: u64, name: &OsStr) -> Result<u64, FsError> {
            // Check parent exists and is a directory
            let inodes = self.inodes.read().unwrap();
            match inodes.get(&parent_inode) {
                None => {
                    return Err(FsError::InodeNotFound {
                        inode: parent_inode,
                    });
                }
                Some((_, file_type)) if *file_type != FileType::Directory => {
                    let (path, _) = inodes.get(&parent_inode).unwrap();
                    return Err(FsError::NotADirectory { path: path.clone() });
                }
                _ => {}
            }
            drop(inodes);

            let name_str = name.to_string_lossy().into_owned();
            self.children
                .read()
                .unwrap()
                .get(&(parent_inode, name_str.clone()))
                .copied()
                .ok_or_else(|| {
                    let parent_path = self
                        .inodes
                        .read()
                        .unwrap()
                        .get(&parent_inode)
                        .map(|(p, _)| p.clone())
                        .unwrap_or_else(|| PathBuf::from("/"));
                    FsError::NotFound {
                        path: parent_path.join(&name_str),
                    }
                })
        }

        fn metadata_by_inode(&self, inode: u64) -> Result<Metadata, FsError> {
            self.inodes
                .read()
                .unwrap()
                .get(&inode)
                .map(|(_, file_type)| Metadata {
                    file_type: *file_type,
                    size: 0,
                    permissions: crate::Permissions::default_file(),
                    created: std::time::SystemTime::UNIX_EPOCH,
                    modified: std::time::SystemTime::UNIX_EPOCH,
                    accessed: std::time::SystemTime::UNIX_EPOCH,
                    inode,
                    nlink: 1,
                })
                .ok_or(FsError::InodeNotFound { inode })
        }
    }

    #[test]
    fn path_to_inode_root() {
        let fs = MockInodeFs::new();
        let inode = fs.path_to_inode(Path::new("/")).unwrap();
        assert_eq!(inode, ROOT_INODE);
    }

    #[test]
    fn path_to_inode_not_found() {
        let fs = MockInodeFs::new();
        let result = fs.path_to_inode(Path::new("/nonexistent"));
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn inode_to_path_root() {
        let fs = MockInodeFs::new();
        let path = fs.inode_to_path(ROOT_INODE).unwrap();
        assert_eq!(path, PathBuf::from("/"));
    }

    #[test]
    fn inode_to_path_not_found() {
        let fs = MockInodeFs::new();
        let result = fs.inode_to_path(9999);
        assert!(matches!(result, Err(FsError::InodeNotFound { .. })));
    }

    #[test]
    fn lookup_child_in_directory() {
        let fs = MockInodeFs::new();
        fs.add_file(Path::new("/file.txt"), 2, FileType::File);

        let inode = fs
            .lookup(ROOT_INODE, std::ffi::OsStr::new("file.txt"))
            .unwrap();
        assert_eq!(inode, 2);
    }

    #[test]
    fn lookup_child_not_found() {
        let fs = MockInodeFs::new();
        let result = fs.lookup(ROOT_INODE, std::ffi::OsStr::new("nonexistent"));
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn lookup_parent_not_directory() {
        let fs = MockInodeFs::new();
        fs.add_file(Path::new("/file.txt"), 2, FileType::File);

        let result = fs.lookup(2, std::ffi::OsStr::new("child"));
        assert!(matches!(result, Err(FsError::NotADirectory { .. })));
    }

    #[test]
    fn metadata_by_inode_returns_metadata() {
        let fs = MockInodeFs::new();
        fs.add_file(Path::new("/file.txt"), 2, FileType::File);

        let meta = fs.metadata_by_inode(2).unwrap();
        assert_eq!(meta.file_type, FileType::File);
        assert_eq!(meta.inode, 2);
    }

    #[test]
    fn metadata_by_inode_not_found() {
        let fs = MockInodeFs::new();
        let result = fs.metadata_by_inode(9999);
        assert!(matches!(result, Err(FsError::InodeNotFound { .. })));
    }

    #[test]
    fn round_trip_path_inode_path() {
        let fs = MockInodeFs::new();
        fs.add_file(Path::new("/subdir"), 2, FileType::Directory);
        fs.add_file(Path::new("/subdir/file.txt"), 3, FileType::File);

        let path = Path::new("/subdir/file.txt");
        let inode = fs.path_to_inode(path).unwrap();
        let recovered_path = fs.inode_to_path(inode).unwrap();
        assert_eq!(path, recovered_path);
    }
}
