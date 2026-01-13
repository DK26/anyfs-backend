//! # FsPath Trait
//!
//! Path canonicalization with a default implementation.
//!
//! ## Responsibility
//! - Provide path canonicalization methods (resolve symlinks, normalize `.`/`..`)
//!
//! ## Dependencies
//! - [`FsRead`] for checking path existence and metadata
//! - [`FsLink`] for symlink resolution
//! - [`FsError`] for error handling
//!
//! ## Usage
//!
//! ```rust
//! use anyfs_backend::{FsPath, FsRead, FsLink};
//! use std::path::Path;
//!
//! // Generic function that works with any FsPath implementation
//! fn resolve<B: FsPath>(backend: &B) -> Result<(), anyfs_backend::FsError> {
//!     // Resolve symlinks and normalize path
//!     let path = backend.canonicalize(Path::new("/some/path/../file.txt"))?;
//!     
//!     // Resolve parent, allow non-existent final component
//!     let new_path = backend.soft_canonicalize(Path::new("/dir/new_file.txt"))?;
//!     Ok(())
//! }
//! ```

use std::path::{Component, Path, PathBuf};

use crate::{FileType, FsError, FsLink, FsRead};

// ============================================================================
// Constants
// ============================================================================

/// Maximum depth for symlink resolution to prevent infinite loops.
const MAX_SYMLINK_DEPTH: usize = 40;

// ============================================================================
// Trait Definition
// ============================================================================

/// Path canonicalization with a default implementation.
///
/// This trait provides methods for resolving paths to their canonical form,
/// handling symlinks and normalizing `.` and `..` components.
///
/// # Blanket Implementation
///
/// This trait has a blanket implementation for any type implementing
/// [`FsRead`] + [`FsLink`], so all backends with symlink support
/// automatically get these methods.
///
/// # Backend Optimization
///
/// Backends can override the default implementation for optimization.
/// For example, `SqliteBackend` could use a single recursive CTE query
/// instead of the iterative component-by-component approach.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsPath, FsRead, FsLink};
/// use std::path::Path;
///
/// // Generic function that works with any FsPath implementation
/// fn resolve<B: FsPath>(backend: &B) -> Result<(), anyfs_backend::FsError> {
///     // Resolve symlinks and normalize path
///     let path = backend.canonicalize(Path::new("/some/path/../file.txt"))?;
///     
///     // Resolve parent, allow non-existent final component
///     let new_path = backend.soft_canonicalize(Path::new("/dir/new_file.txt"))?;
///     Ok(())
/// }
/// ```
pub trait FsPath: FsRead + FsLink {
    /// Resolve all symlinks and normalize path (`.`, `..`).
    ///
    /// All path components must exist. Returns error if any component
    /// is missing or a symlink loop is detected.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to canonicalize
    ///
    /// # Returns
    ///
    /// The fully resolved canonical path.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] - A component doesn't exist
    /// - [`FsError::InvalidData`] - Symlink loop detected (exceeded max depth)
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyfs_backend::FsPath;
    /// use std::path::{Path, PathBuf};
    ///
    /// // Generic function that demonstrates canonicalize
    /// fn resolve_link<B: FsPath>(backend: &B) -> Result<PathBuf, anyfs_backend::FsError> {
    ///     // Given: /link -> /target, /target/file.txt exists
    ///     let path = backend.canonicalize(Path::new("/link/file.txt"))?;
    ///     // Result: PathBuf::from("/target/file.txt")
    ///     Ok(path)
    /// }
    /// ```
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FsError> {
        default_canonicalize(self, path)
    }

    /// Like [`canonicalize`](Self::canonicalize), but allows non-existent final component.
    ///
    /// Resolves parent path fully, appends final component lexically.
    /// This is useful for `write()` operations where the target file
    /// doesn't exist yet.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to soft-canonicalize
    ///
    /// # Returns
    ///
    /// The resolved path with the final component appended lexically.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] - A parent component doesn't exist
    /// - [`FsError::InvalidData`] - Symlink loop detected
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyfs_backend::FsPath;
    /// use std::path::{Path, PathBuf};
    ///
    /// // Generic function that demonstrates soft_canonicalize
    /// fn resolve_new_file<B: FsPath>(backend: &B) -> Result<PathBuf, anyfs_backend::FsError> {
    ///     // Given: /dir exists, /dir/new_file.txt does NOT exist
    ///     let path = backend.soft_canonicalize(Path::new("/dir/new_file.txt"))?;
    ///     // Result: PathBuf::from("/dir/new_file.txt")
    ///     Ok(path)
    /// }
    /// ```
    fn soft_canonicalize(&self, path: &Path) -> Result<PathBuf, FsError> {
        default_soft_canonicalize(self, path)
    }
}

// Blanket implementation - any FsRead + FsLink gets FsPath for free
impl<T: FsRead + FsLink> FsPath for T {}

// ============================================================================
// Default Implementations
// ============================================================================

/// Default implementation of canonicalize using iterative resolution.
///
/// Walks the path component by component, following symlinks and
/// resolving `.` and `..` components.
fn default_canonicalize<F: FsRead + FsLink + ?Sized>(
    fs: &F,
    path: &Path,
) -> Result<PathBuf, FsError> {
    resolve_path_internal(fs, path, 0, true)
}

/// Default implementation of soft_canonicalize.
///
/// Like canonicalize, but allows the final component to not exist.
fn default_soft_canonicalize<F: FsRead + FsLink + ?Sized>(
    fs: &F,
    path: &Path,
) -> Result<PathBuf, FsError> {
    // Get the parent and final component
    let parent = path.parent();
    let file_name = path.file_name();

    match (parent, file_name) {
        (Some(parent_path), Some(name)) if !parent_path.as_os_str().is_empty() => {
            // Resolve the parent path fully
            let resolved_parent = resolve_path_internal(fs, parent_path, 0, true)?;
            // Append the final component lexically
            Ok(resolved_parent.join(name))
        }
        (None, Some(_)) | (Some(_), Some(_)) => {
            // Just a filename or root + filename, return as-is normalized
            normalize_path(path)
        }
        (_, None) => {
            // No filename component (e.g., "/" or empty) - just canonicalize
            default_canonicalize(fs, path)
        }
    }
}

/// Internal path resolution with symlink depth tracking.
fn resolve_path_internal<F: FsRead + FsLink + ?Sized>(
    fs: &F,
    path: &Path,
    depth: usize,
    require_exists: bool,
) -> Result<PathBuf, FsError> {
    if depth > MAX_SYMLINK_DEPTH {
        return Err(FsError::InvalidData {
            path: path.to_path_buf(),
            details: format!("symlink loop detected (exceeded max depth of {MAX_SYMLINK_DEPTH})"),
        });
    }

    let mut resolved = PathBuf::new();

    for component in path.components() {
        match component {
            Component::RootDir => {
                resolved = PathBuf::from("/");
            }
            Component::CurDir => {
                // `.` - skip, don't change resolved path
            }
            Component::ParentDir => {
                // `..` - go up one level
                resolved.pop();
                // Ensure we don't go above root
                if resolved.as_os_str().is_empty() {
                    resolved = PathBuf::from("/");
                }
            }
            Component::Normal(name) => {
                resolved.push(name);

                // Check if this component is a symlink
                match fs.symlink_metadata(&resolved) {
                    Ok(meta) => {
                        if meta.file_type == FileType::Symlink {
                            // Read the symlink target
                            let target = fs.read_link(&resolved)?;

                            // Remove the symlink from resolved path
                            resolved.pop();

                            // Resolve the target path
                            let target_resolved = if target.is_absolute() {
                                resolve_path_internal(fs, &target, depth + 1, require_exists)?
                            } else {
                                // Relative symlink - resolve relative to current resolved path
                                let full_target = resolved.join(&target);
                                resolve_path_internal(fs, &full_target, depth + 1, require_exists)?
                            };

                            resolved = target_resolved;
                        }
                        // If not a symlink, keep it in resolved path
                    }
                    Err(FsError::NotFound { .. }) if !require_exists => {
                        // Component doesn't exist but we're in soft mode
                        // Keep it in the path
                    }
                    Err(e) => return Err(e),
                }
            }
            Component::Prefix(_) => {
                // Windows prefix handling - for cross-platform support
                // Virtual backends use Unix-style paths internally
                resolved.push(component);
            }
        }
    }

    // Ensure we have at least root
    if resolved.as_os_str().is_empty() {
        resolved = PathBuf::from("/");
    }

    // Final existence check for canonicalize (not soft)
    if require_exists && !fs.exists(&resolved)? {
        return Err(FsError::NotFound { path: resolved });
    }

    Ok(resolved)
}

/// Simple lexical path normalization without filesystem access.
///
/// Handles `.`, `..`, and multiple slashes but does NOT follow symlinks.
fn normalize_path(path: &Path) -> Result<PathBuf, FsError> {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::RootDir => {
                normalized = PathBuf::from("/");
            }
            Component::CurDir => {
                // Skip `.`
            }
            Component::ParentDir => {
                normalized.pop();
                if normalized.as_os_str().is_empty() {
                    normalized = PathBuf::from("/");
                }
            }
            Component::Normal(name) => {
                normalized.push(name);
            }
            Component::Prefix(prefix) => {
                normalized.push(prefix.as_os_str());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        normalized = PathBuf::from("/");
    }

    Ok(normalized)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FsDir, FsWrite, Metadata, Permissions, ReadDirIter};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::sync::RwLock;
    use std::time::SystemTime;

    // Mock filesystem with configurable entries
    struct MockFs {
        entries: RwLock<HashMap<PathBuf, MockEntry>>,
    }

    #[derive(Clone)]
    enum MockEntry {
        File,
        Directory,
        Symlink(PathBuf),
    }

    impl MockFs {
        fn new() -> Self {
            let mut entries = HashMap::new();
            // Root always exists
            entries.insert(PathBuf::from("/"), MockEntry::Directory);
            Self {
                entries: RwLock::new(entries),
            }
        }

        fn add_file(&self, path: impl Into<PathBuf>) {
            self.entries
                .write()
                .unwrap()
                .insert(path.into(), MockEntry::File);
        }

        fn add_dir(&self, path: impl Into<PathBuf>) {
            self.entries
                .write()
                .unwrap()
                .insert(path.into(), MockEntry::Directory);
        }

        fn add_symlink(&self, path: impl Into<PathBuf>, target: impl Into<PathBuf>) {
            self.entries
                .write()
                .unwrap()
                .insert(path.into(), MockEntry::Symlink(target.into()));
        }
    }

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

        fn exists(&self, path: &Path) -> Result<bool, FsError> {
            Ok(self.entries.read().unwrap().contains_key(path))
        }

        fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
            let entries = self.entries.read().unwrap();
            match entries.get(path) {
                Some(entry) => {
                    let file_type = match entry {
                        MockEntry::File => FileType::File,
                        MockEntry::Directory => FileType::Directory,
                        MockEntry::Symlink(target) => {
                            // Follow symlink for metadata - clone target first
                            let target = target.clone();
                            drop(entries);
                            return self.metadata(&target);
                        }
                    };
                    Ok(Metadata {
                        file_type,
                        size: 0,
                        permissions: Permissions::default_file(),
                        created: SystemTime::UNIX_EPOCH,
                        modified: SystemTime::UNIX_EPOCH,
                        accessed: SystemTime::UNIX_EPOCH,
                        inode: 1,
                        nlink: 1,
                    })
                }
                None => Err(FsError::NotFound {
                    path: path.to_path_buf(),
                }),
            }
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

    impl FsLink for MockFs {
        fn symlink(&self, _target: &Path, _link: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn hard_link(&self, _original: &Path, _link: &Path) -> Result<(), FsError> {
            Ok(())
        }

        fn read_link(&self, path: &Path) -> Result<PathBuf, FsError> {
            let entries = self.entries.read().unwrap();
            match entries.get(path) {
                Some(MockEntry::Symlink(target)) => Ok(target.clone()),
                Some(_) => Err(FsError::InvalidData {
                    path: path.to_path_buf(),
                    details: "not a symlink".to_string(),
                }),
                None => Err(FsError::NotFound {
                    path: path.to_path_buf(),
                }),
            }
        }

        fn symlink_metadata(&self, path: &Path) -> Result<Metadata, FsError> {
            let entries = self.entries.read().unwrap();
            match entries.get(path) {
                Some(entry) => {
                    let file_type = match entry {
                        MockEntry::File => FileType::File,
                        MockEntry::Directory => FileType::Directory,
                        MockEntry::Symlink(_) => FileType::Symlink,
                    };
                    Ok(Metadata {
                        file_type,
                        size: 0,
                        permissions: Permissions::default_file(),
                        created: SystemTime::UNIX_EPOCH,
                        modified: SystemTime::UNIX_EPOCH,
                        accessed: SystemTime::UNIX_EPOCH,
                        inode: 1,
                        nlink: 1,
                    })
                }
                None => Err(FsError::NotFound {
                    path: path.to_path_buf(),
                }),
            }
        }
    }

    #[test]
    fn fs_path_blanket_impl_works() {
        // Verify the blanket impl works
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/test"));
        fs.add_file(PathBuf::from("/test/file.txt"));

        // Should be able to call FsPath methods on MockFs
        let result = fs.canonicalize(Path::new("/test/file.txt"));
        assert!(result.is_ok());
    }

    #[test]
    fn canonicalize_simple_path() {
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/dir"));
        fs.add_file(PathBuf::from("/dir/file.txt"));

        let result = fs.canonicalize(Path::new("/dir/file.txt"));
        assert_eq!(result.unwrap(), PathBuf::from("/dir/file.txt"));
    }

    #[test]
    fn canonicalize_resolves_dot() {
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/dir"));
        fs.add_file(PathBuf::from("/dir/file.txt"));

        let result = fs.canonicalize(Path::new("/dir/./file.txt"));
        assert_eq!(result.unwrap(), PathBuf::from("/dir/file.txt"));
    }

    #[test]
    fn canonicalize_resolves_dotdot() {
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/dir"));
        fs.add_dir(PathBuf::from("/dir/sub"));
        fs.add_file(PathBuf::from("/dir/file.txt"));

        let result = fs.canonicalize(Path::new("/dir/sub/../file.txt"));
        assert_eq!(result.unwrap(), PathBuf::from("/dir/file.txt"));
    }

    #[test]
    fn canonicalize_follows_symlink() {
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/target"));
        fs.add_file(PathBuf::from("/target/file.txt"));
        fs.add_symlink(PathBuf::from("/link"), PathBuf::from("/target"));

        let result = fs.canonicalize(Path::new("/link/file.txt"));
        assert_eq!(result.unwrap(), PathBuf::from("/target/file.txt"));
    }

    #[test]
    fn canonicalize_follows_relative_symlink() {
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/dir"));
        fs.add_dir(PathBuf::from("/dir/target"));
        fs.add_file(PathBuf::from("/dir/target/file.txt"));
        fs.add_symlink(PathBuf::from("/dir/link"), PathBuf::from("target"));

        let result = fs.canonicalize(Path::new("/dir/link/file.txt"));
        assert_eq!(result.unwrap(), PathBuf::from("/dir/target/file.txt"));
    }

    #[test]
    fn canonicalize_detects_symlink_loop() {
        let fs = MockFs::new();
        fs.add_symlink(PathBuf::from("/loop1"), PathBuf::from("/loop2"));
        fs.add_symlink(PathBuf::from("/loop2"), PathBuf::from("/loop1"));

        let result = fs.canonicalize(Path::new("/loop1"));
        assert!(result.is_err());
        if let Err(FsError::InvalidData { details, .. }) = result {
            assert!(details.contains("symlink loop"));
        } else {
            panic!("Expected InvalidData error for symlink loop");
        }
    }

    #[test]
    fn canonicalize_not_found() {
        let fs = MockFs::new();

        let result = fs.canonicalize(Path::new("/nonexistent"));
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn soft_canonicalize_allows_nonexistent_final() {
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/dir"));

        // /dir exists, but /dir/new_file.txt does not
        let result = fs.soft_canonicalize(Path::new("/dir/new_file.txt"));
        assert_eq!(result.unwrap(), PathBuf::from("/dir/new_file.txt"));
    }

    #[test]
    fn soft_canonicalize_resolves_parent_symlink() {
        let fs = MockFs::new();
        fs.add_dir(PathBuf::from("/target"));
        fs.add_symlink(PathBuf::from("/link"), PathBuf::from("/target"));

        // /link -> /target, so /link/new.txt -> /target/new.txt
        let result = fs.soft_canonicalize(Path::new("/link/new.txt"));
        assert_eq!(result.unwrap(), PathBuf::from("/target/new.txt"));
    }

    #[test]
    fn soft_canonicalize_fails_for_nonexistent_parent() {
        let fs = MockFs::new();
        // /nonexistent doesn't exist

        let result = fs.soft_canonicalize(Path::new("/nonexistent/file.txt"));
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn canonicalize_root() {
        let fs = MockFs::new();

        let result = fs.canonicalize(Path::new("/"));
        assert_eq!(result.unwrap(), PathBuf::from("/"));
    }

    #[test]
    fn normalize_path_handles_dots() {
        let result = normalize_path(Path::new("/a/./b/../c"));
        assert_eq!(result.unwrap(), PathBuf::from("/a/c"));
    }

    #[test]
    fn normalize_path_handles_root() {
        let result = normalize_path(Path::new("/"));
        assert_eq!(result.unwrap(), PathBuf::from("/"));
    }
}
