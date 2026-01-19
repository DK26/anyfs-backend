//! Complete in-memory filesystem reference implementation.
//!
//! This example provides a full implementation of ALL anyfs-backend traits
//! up to `FsPosix`. Use this as a reference when implementing your own
//! filesystem backend.
//!
//! Run with: `cargo run --example inmemory_fs`
//!
//! This implementation is suitable as a starting point for:
//! - Testing and mocking
//! - In-memory caching layers
//! - Learning how each trait method should behave

use anyfs_backend::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::SystemTime;

// =============================================================================
// Complete In-Memory Filesystem Implementation
// =============================================================================

/// A complete in-memory filesystem implementing all traits up to FsPosix.
///
/// ## Thread Safety
///
/// This implementation uses `RwLock` for interior mutability, making it
/// safe to use from multiple threads. All trait methods take `&self`,
/// following the anyfs-backend design requirement.
///
/// ## Trait Hierarchy
///
/// This type implements:
/// - Layer 1: `Fs` = `FsRead` + `FsWrite` + `FsDir`
/// - Layer 2: `FsFull` = `Fs` + `FsLink` + `FsPermissions` + `FsSync` + `FsStats`
/// - Layer 3: `FsFuse` = `FsFull` + `FsInode`
/// - Layer 4: `FsPosix` = `FsFuse` + `FsHandles` + `FsLock` + `FsXattr`
pub struct InMemoryFs {
    /// File contents, keyed by path
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,

    /// Set of directories
    dirs: RwLock<std::collections::HashSet<PathBuf>>,

    /// Symlinks: link_path -> target_path
    symlinks: RwLock<HashMap<PathBuf, PathBuf>>,

    /// Extended attributes: path -> (name -> value)
    xattrs: RwLock<HashMap<PathBuf, HashMap<String, Vec<u8>>>>,

    /// Inode mapping: path -> inode number
    inodes: RwLock<HashMap<PathBuf, u64>>,

    /// Reverse inode mapping: inode number -> path
    inode_to_path: RwLock<HashMap<u64, PathBuf>>,

    /// Open file handles: handle_id -> (path, flags)
    handles: RwLock<HashMap<u64, OpenFile>>,

    /// File locks: handle_id -> lock state
    locks: RwLock<HashMap<u64, LockState>>,

    /// Counter for generating unique inode numbers
    next_inode: AtomicU64,

    /// Counter for generating unique handle IDs
    next_handle: AtomicU64,
}

/// Information about an open file.
struct OpenFile {
    path: PathBuf,
    flags: OpenFlags,
}

/// Current lock state for a file handle.
#[derive(Clone, Copy, PartialEq)]
enum LockState {
    Unlocked,
    Shared(usize), // Count of shared locks
    Exclusive,
}

impl InMemoryFs {
    /// Create a new empty in-memory filesystem.
    ///
    /// The root directory `/` is created automatically.
    pub fn new() -> Self {
        let fs = Self {
            files: RwLock::new(HashMap::new()),
            dirs: RwLock::new(std::collections::HashSet::new()),
            symlinks: RwLock::new(HashMap::new()),
            xattrs: RwLock::new(HashMap::new()),
            inodes: RwLock::new(HashMap::new()),
            inode_to_path: RwLock::new(HashMap::new()),
            handles: RwLock::new(HashMap::new()),
            locks: RwLock::new(HashMap::new()),
            next_inode: AtomicU64::new(2), // 1 is reserved for root
            next_handle: AtomicU64::new(1),
        };

        // Create root directory with inode 1
        fs.dirs.write().unwrap().insert(PathBuf::from("/"));
        fs.assign_inode(Path::new("/"));

        fs
    }

    /// Assign an inode to a path (or return existing inode).
    fn assign_inode(&self, path: &Path) -> u64 {
        let mut inodes = self.inodes.write().unwrap();

        // Return existing inode if path already has one
        if let Some(&inode) = inodes.get(path) {
            return inode;
        }

        // Root always gets inode 1
        let inode = if path == Path::new("/") {
            ROOT_INODE
        } else {
            self.next_inode.fetch_add(1, Ordering::SeqCst)
        };

        // Store bidirectional mapping
        inodes.insert(path.to_path_buf(), inode);
        self.inode_to_path
            .write()
            .unwrap()
            .insert(inode, path.to_path_buf());

        inode
    }

    /// Get the file type at a path (without following symlinks).
    fn get_file_type(&self, path: &Path) -> Option<FileType> {
        if self.symlinks.read().unwrap().contains_key(path) {
            Some(FileType::Symlink)
        } else if self.dirs.read().unwrap().contains(path) {
            Some(FileType::Directory)
        } else if self.files.read().unwrap().contains_key(path) {
            Some(FileType::File)
        } else {
            None
        }
    }
}

impl Default for InMemoryFs {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Layer 1: FsRead - Reading files and metadata
// =============================================================================

impl FsRead for InMemoryFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.files
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })
    }

    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|_| FsError::InvalidData {
            path: path.to_path_buf(),
            details: "file contents are not valid UTF-8".into(),
        })
    }

    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        let data = self.read(path)?;
        let start = offset as usize;

        if start >= data.len() {
            return Ok(Vec::new());
        }

        let end = (start + len).min(data.len());
        Ok(data[start..end].to_vec())
    }

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        Ok(self.get_file_type(path).is_some())
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let file_type = self.get_file_type(path).ok_or_else(|| FsError::NotFound {
            path: path.to_path_buf(),
        })?;

        let size = if file_type == FileType::File {
            self.files
                .read()
                .unwrap()
                .get(path)
                .map(|d| d.len() as u64)
                .unwrap_or(0)
        } else {
            0
        };

        let inode = self.inodes.read().unwrap().get(path).copied().unwrap_or(0);

        Ok(Metadata {
            file_type,
            size,
            permissions: Permissions::default_file(),
            created: SystemTime::UNIX_EPOCH,
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            inode,
            nlink: 1,
        })
    }

    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
        let data = self.read(path)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }
}

// =============================================================================
// Layer 1: FsWrite - Writing and modifying files
// =============================================================================

impl FsWrite for InMemoryFs {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.assign_inode(path);
        self.files
            .write()
            .unwrap()
            .insert(path.to_path_buf(), data.to_vec());
        Ok(())
    }

    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        let mut files = self.files.write().unwrap();
        files
            .entry(path.to_path_buf())
            .or_default()
            .extend_from_slice(data);
        drop(files);
        self.assign_inode(path);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .remove(path)
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })?;

        // Clean up inode mapping
        if let Some(inode) = self.inodes.write().unwrap().remove(path) {
            self.inode_to_path.write().unwrap().remove(&inode);
        }

        // Clean up xattrs
        self.xattrs.write().unwrap().remove(path);

        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let mut files = self.files.write().unwrap();
        let data = files.remove(from).ok_or_else(|| FsError::NotFound {
            path: from.to_path_buf(),
        })?;
        files.insert(to.to_path_buf(), data);
        drop(files);

        // Update inode mappings
        if let Some(inode) = self.inodes.write().unwrap().remove(from) {
            self.inodes.write().unwrap().insert(to.to_path_buf(), inode);
            self.inode_to_path
                .write()
                .unwrap()
                .insert(inode, to.to_path_buf());
        } else {
            self.assign_inode(to);
        }

        // Move xattrs
        if let Some(attrs) = self.xattrs.write().unwrap().remove(from) {
            self.xattrs.write().unwrap().insert(to.to_path_buf(), attrs);
        }

        Ok(())
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let data = self.read(from)?;
        self.write(to, &data)
    }

    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> {
        let mut files = self.files.write().unwrap();
        let data = files.get_mut(path).ok_or_else(|| FsError::NotFound {
            path: path.to_path_buf(),
        })?;
        data.resize(size as usize, 0);
        Ok(())
    }

    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        // Create file if it doesn't exist
        if !self.files.read().unwrap().contains_key(path) {
            self.write(path, &[])?;
        }

        // Return a cursor that writes to a buffer
        // Note: In a real implementation, this would write back on drop
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }
}

// =============================================================================
// Layer 1: FsDir - Directory operations
// =============================================================================

impl FsDir for InMemoryFs {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        if !self.dirs.read().unwrap().contains(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        let mut entries = Vec::new();

        // Collect files in this directory
        for (file_path, data) in self.files.read().unwrap().iter() {
            if let Some(parent) = file_path.parent() {
                if parent == path {
                    if let Some(name) = file_path.file_name() {
                        entries.push(Ok(DirEntry {
                            name: name.to_string_lossy().into_owned(),
                            path: file_path.clone(),
                            file_type: FileType::File,
                            size: data.len() as u64,
                            inode: self
                                .inodes
                                .read()
                                .unwrap()
                                .get(file_path)
                                .copied()
                                .unwrap_or(0),
                        }));
                    }
                }
            }
        }

        // Collect subdirectories
        for dir_path in self.dirs.read().unwrap().iter() {
            if let Some(parent) = dir_path.parent() {
                if parent == path && dir_path != path {
                    if let Some(name) = dir_path.file_name() {
                        entries.push(Ok(DirEntry {
                            name: name.to_string_lossy().into_owned(),
                            path: dir_path.clone(),
                            file_type: FileType::Directory,
                            size: 0,
                            inode: self
                                .inodes
                                .read()
                                .unwrap()
                                .get(dir_path)
                                .copied()
                                .unwrap_or(0),
                        }));
                    }
                }
            }
        }

        // Collect symlinks
        for link_path in self.symlinks.read().unwrap().keys() {
            if let Some(parent) = link_path.parent() {
                if parent == path {
                    if let Some(name) = link_path.file_name() {
                        entries.push(Ok(DirEntry {
                            name: name.to_string_lossy().into_owned(),
                            path: link_path.clone(),
                            file_type: FileType::Symlink,
                            size: 0,
                            inode: self
                                .inodes
                                .read()
                                .unwrap()
                                .get(link_path)
                                .copied()
                                .unwrap_or(0),
                        }));
                    }
                }
            }
        }

        Ok(ReadDirIter::from_vec(entries))
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        let mut dirs = self.dirs.write().unwrap();

        if dirs.contains(path) {
            return Err(FsError::AlreadyExists {
                path: path.to_path_buf(),
                operation: "create_dir",
            });
        }

        // Check parent exists
        if let Some(parent) = path.parent() {
            if parent != Path::new("") && parent != Path::new("/") && !dirs.contains(parent) {
                return Err(FsError::NotFound {
                    path: parent.to_path_buf(),
                });
            }
        }

        dirs.insert(path.to_path_buf());
        drop(dirs);
        self.assign_inode(path);

        Ok(())
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        let mut current = PathBuf::new();

        for component in path.components() {
            current.push(component);

            let mut dirs = self.dirs.write().unwrap();
            if !dirs.contains(&current) {
                dirs.insert(current.clone());
                drop(dirs);
                self.assign_inode(&current);
            }
        }

        Ok(())
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        // Check if directory is empty
        let has_files = self
            .files
            .read()
            .unwrap()
            .keys()
            .any(|p| p.parent() == Some(path));

        let has_subdirs = self
            .dirs
            .read()
            .unwrap()
            .iter()
            .any(|p| p.parent() == Some(path) && p != path);

        if has_files || has_subdirs {
            return Err(FsError::DirectoryNotEmpty {
                path: path.to_path_buf(),
            });
        }

        if !self.dirs.write().unwrap().remove(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        // Clean up inode
        if let Some(inode) = self.inodes.write().unwrap().remove(path) {
            self.inode_to_path.write().unwrap().remove(&inode);
        }

        Ok(())
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        // Remove all files under this path
        self.files
            .write()
            .unwrap()
            .retain(|p, _| !p.starts_with(path));

        // Remove all subdirectories under this path
        self.dirs.write().unwrap().retain(|p| !p.starts_with(path));

        // Remove all symlinks under this path
        self.symlinks
            .write()
            .unwrap()
            .retain(|p, _| !p.starts_with(path));

        // Clean up inodes
        let paths_to_remove: Vec<_> = self
            .inodes
            .read()
            .unwrap()
            .keys()
            .filter(|p| p.starts_with(path))
            .cloned()
            .collect();

        for p in paths_to_remove {
            if let Some(inode) = self.inodes.write().unwrap().remove(&p) {
                self.inode_to_path.write().unwrap().remove(&inode);
            }
        }

        Ok(())
    }
}

// =============================================================================
// Layer 2: FsLink - Symbolic and hard links
// =============================================================================

impl FsLink for InMemoryFs {
    fn symlink(&self, target: &Path, link: &Path) -> Result<(), FsError> {
        if self.get_file_type(link).is_some() {
            return Err(FsError::AlreadyExists {
                path: link.to_path_buf(),
                operation: "symlink",
            });
        }

        self.symlinks
            .write()
            .unwrap()
            .insert(link.to_path_buf(), target.to_path_buf());
        self.assign_inode(link);

        Ok(())
    }

    fn hard_link(&self, original: &Path, link: &Path) -> Result<(), FsError> {
        // For in-memory fs, we just copy the data (true hard links would share data)
        let data = self.read(original)?;
        self.write(link, &data)?;

        // In a real implementation, you'd update nlink count
        Ok(())
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf, FsError> {
        self.symlinks
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| FsError::InvalidData {
                path: path.to_path_buf(),
                details: "not a symbolic link".into(),
            })
    }

    fn symlink_metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let file_type = self.get_file_type(path).ok_or_else(|| FsError::NotFound {
            path: path.to_path_buf(),
        })?;

        Ok(Metadata {
            file_type,
            size: 0,
            permissions: Permissions::default_file(),
            created: SystemTime::UNIX_EPOCH,
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            inode: self.inodes.read().unwrap().get(path).copied().unwrap_or(0),
            nlink: 1,
        })
    }
}

// =============================================================================
// Layer 2: FsPermissions - Permission management
// =============================================================================

impl FsPermissions for InMemoryFs {
    fn set_permissions(&self, path: &Path, _perm: Permissions) -> Result<(), FsError> {
        // Verify path exists
        if self.get_file_type(path).is_none() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        // In a real implementation, you'd store and enforce permissions
        Ok(())
    }
}

// =============================================================================
// Layer 2: FsSync - Synchronization operations
// =============================================================================

impl FsSync for InMemoryFs {
    fn sync(&self) -> Result<(), FsError> {
        // In-memory filesystem is always synchronized
        Ok(())
    }

    fn fsync(&self, path: &Path) -> Result<(), FsError> {
        // Verify path exists
        if self.get_file_type(path).is_none() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        // In-memory filesystem is always synchronized
        Ok(())
    }
}

// =============================================================================
// Layer 2: FsStats - Filesystem statistics
// =============================================================================

impl FsStats for InMemoryFs {
    fn statfs(&self) -> Result<StatFs, FsError> {
        let files = self.files.read().unwrap();
        let used_bytes: u64 = files.values().map(|d| d.len() as u64).sum();
        let used_inodes = files.len() + self.dirs.read().unwrap().len();

        Ok(StatFs {
            total_bytes: 1024 * 1024 * 100, // 100 MB
            used_bytes,
            available_bytes: 1024 * 1024 * 100 - used_bytes,
            total_inodes: 100_000,
            used_inodes: used_inodes as u64,
            available_inodes: 100_000 - used_inodes as u64,
            block_size: 4096,
            max_name_len: 255,
        })
    }
}

// =============================================================================
// Layer 3: FsInode - Inode-based operations (for FUSE)
// =============================================================================

impl FsInode for InMemoryFs {
    fn path_to_inode(&self, path: &Path) -> Result<u64, FsError> {
        self.inodes
            .read()
            .unwrap()
            .get(path)
            .copied()
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })
    }

    fn inode_to_path(&self, inode: u64) -> Result<PathBuf, FsError> {
        self.inode_to_path
            .read()
            .unwrap()
            .get(&inode)
            .cloned()
            .ok_or(FsError::InodeNotFound { inode })
    }

    fn lookup(&self, parent_inode: u64, name: &OsStr) -> Result<u64, FsError> {
        let parent_path = self.inode_to_path(parent_inode)?;
        let child_path = parent_path.join(name);
        self.path_to_inode(&child_path)
    }

    fn metadata_by_inode(&self, inode: u64) -> Result<Metadata, FsError> {
        let path = self.inode_to_path(inode)?;
        self.metadata(&path)
    }
}

// =============================================================================
// Layer 4: FsHandles - Handle-based I/O
// =============================================================================

impl FsHandles for InMemoryFs {
    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Handle, FsError> {
        // Create file if requested
        if flags.create {
            if !self.files.read().unwrap().contains_key(path) {
                self.write(path, &[])?;
            }
        } else if !self.files.read().unwrap().contains_key(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        let handle_id = self.next_handle.fetch_add(1, Ordering::SeqCst);

        self.handles.write().unwrap().insert(
            handle_id,
            OpenFile {
                path: path.to_path_buf(),
                flags,
            },
        );

        Ok(Handle(handle_id))
    }

    fn read_at(&self, handle: Handle, buf: &mut [u8], offset: u64) -> Result<usize, FsError> {
        let handles = self.handles.read().unwrap();
        let open_file = handles
            .get(&handle.0)
            .ok_or(FsError::InvalidHandle { handle })?;

        // Check read permission
        if !open_file.flags.read {
            return Err(FsError::PermissionDenied {
                path: open_file.path.clone(),
                operation: "read",
            });
        }

        let files = self.files.read().unwrap();
        let data = files.get(&open_file.path).ok_or(FsError::NotFound {
            path: open_file.path.clone(),
        })?;

        let start = offset as usize;
        if start >= data.len() {
            return Ok(0);
        }

        let end = (start + buf.len()).min(data.len());
        let bytes_read = end - start;
        buf[..bytes_read].copy_from_slice(&data[start..end]);

        Ok(bytes_read)
    }

    fn write_at(&self, handle: Handle, data: &[u8], offset: u64) -> Result<usize, FsError> {
        let path = {
            let handles = self.handles.read().unwrap();
            let open_file = handles
                .get(&handle.0)
                .ok_or(FsError::InvalidHandle { handle })?;

            // Check write permission
            if !open_file.flags.write {
                return Err(FsError::PermissionDenied {
                    path: open_file.path.clone(),
                    operation: "write",
                });
            }

            open_file.path.clone()
        };

        let mut files = self.files.write().unwrap();
        let file_data = files.entry(path).or_default();

        let start = offset as usize;
        if start + data.len() > file_data.len() {
            file_data.resize(start + data.len(), 0);
        }
        file_data[start..start + data.len()].copy_from_slice(data);

        Ok(data.len())
    }

    fn close(&self, handle: Handle) -> Result<(), FsError> {
        // Release any locks
        self.locks.write().unwrap().remove(&handle.0);

        // Remove handle
        self.handles
            .write()
            .unwrap()
            .remove(&handle.0)
            .map(|_| ())
            .ok_or(FsError::InvalidHandle { handle })
    }
}

// =============================================================================
// Layer 4: FsLock - File locking
// =============================================================================

impl FsLock for InMemoryFs {
    fn lock(&self, handle: Handle, lock_type: LockType) -> Result<(), FsError> {
        if !self.handles.read().unwrap().contains_key(&handle.0) {
            return Err(FsError::InvalidHandle { handle });
        }

        let mut locks = self.locks.write().unwrap();
        let state = locks.entry(handle.0).or_insert(LockState::Unlocked);

        match (*state, lock_type) {
            (LockState::Unlocked, LockType::Shared) => {
                *state = LockState::Shared(1);
                Ok(())
            }
            (LockState::Unlocked, LockType::Exclusive) => {
                *state = LockState::Exclusive;
                Ok(())
            }
            (LockState::Shared(n), LockType::Shared) => {
                *state = LockState::Shared(n + 1);
                Ok(())
            }
            _ => Err(FsError::Conflict {
                path: PathBuf::new(),
            }),
        }
    }

    fn try_lock(&self, handle: Handle, lock_type: LockType) -> Result<bool, FsError> {
        if !self.handles.read().unwrap().contains_key(&handle.0) {
            return Err(FsError::InvalidHandle { handle });
        }

        let mut locks = self.locks.write().unwrap();
        let state = locks.entry(handle.0).or_insert(LockState::Unlocked);

        match (*state, lock_type) {
            (LockState::Unlocked, LockType::Shared) => {
                *state = LockState::Shared(1);
                Ok(true)
            }
            (LockState::Unlocked, LockType::Exclusive) => {
                *state = LockState::Exclusive;
                Ok(true)
            }
            (LockState::Shared(n), LockType::Shared) => {
                *state = LockState::Shared(n + 1);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn unlock(&self, handle: Handle) -> Result<(), FsError> {
        if !self.handles.read().unwrap().contains_key(&handle.0) {
            return Err(FsError::InvalidHandle { handle });
        }

        let mut locks = self.locks.write().unwrap();
        if let Some(state) = locks.get_mut(&handle.0) {
            match *state {
                LockState::Shared(1) => *state = LockState::Unlocked,
                LockState::Shared(n) => *state = LockState::Shared(n - 1),
                LockState::Exclusive => *state = LockState::Unlocked,
                LockState::Unlocked => {}
            }
        }

        Ok(())
    }
}

// =============================================================================
// Layer 4: FsXattr - Extended attributes
// =============================================================================

impl FsXattr for InMemoryFs {
    fn get_xattr(&self, path: &Path, name: &str) -> Result<Vec<u8>, FsError> {
        // Check file exists
        if self.get_file_type(path).is_none() {
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
        // Check file exists
        if self.get_file_type(path).is_none() {
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
        // Check file exists
        if self.get_file_type(path).is_none() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        self.xattrs
            .write()
            .unwrap()
            .get_mut(path)
            .and_then(|attrs| attrs.remove(name))
            .ok_or_else(|| FsError::XattrNotFound {
                path: path.to_path_buf(),
                name: name.to_string(),
            })?;

        Ok(())
    }

    fn list_xattr(&self, path: &Path) -> Result<Vec<String>, FsError> {
        // Check file exists
        if self.get_file_type(path).is_none() {
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

// =============================================================================
// Main: Demonstrate the complete implementation
// =============================================================================

fn main() {
    println!("=== In-Memory Filesystem Reference Implementation ===\n");

    let fs = InMemoryFs::new();

    // --- Layer 1: Fs operations ---
    println!("--- Layer 1: Fs (FsRead + FsWrite + FsDir) ---\n");

    fs.create_dir_all(Path::new("/project/src")).unwrap();
    fs.write(Path::new("/project/README.md"), b"# My Project\n\nHello!")
        .unwrap();
    fs.write(Path::new("/project/src/main.rs"), b"fn main() {}")
        .unwrap();

    println!("Created project structure:");
    for entry in fs.read_dir(Path::new("/project")).unwrap() {
        let entry = entry.unwrap();
        println!("  {} ({:?})", entry.name, entry.file_type);
    }

    let readme = fs.read_to_string(Path::new("/project/README.md")).unwrap();
    println!("\nREADME.md content:\n{readme}");

    // --- Layer 2: FsFull operations ---
    println!("\n--- Layer 2: FsFull (+FsLink, +FsStats, +FsSync) ---\n");

    fs.symlink(Path::new("/project/README.md"), Path::new("/project/docs"))
        .unwrap();
    let target = fs.read_link(Path::new("/project/docs")).unwrap();
    println!("Symlink /project/docs -> {}", target.display());

    let stats = fs.statfs().unwrap();
    println!(
        "Filesystem: {} bytes used, {} available",
        stats.used_bytes, stats.available_bytes
    );

    fs.sync().unwrap();
    println!("Filesystem synced");

    // --- Layer 3: FsFuse operations ---
    println!("\n--- Layer 3: FsFuse (+FsInode) ---\n");

    let root_inode = fs.path_to_inode(Path::new("/")).unwrap();
    println!("Root inode: {root_inode} (should be {})", ROOT_INODE);

    let project_inode = fs.lookup(root_inode, OsStr::new("project")).unwrap();
    println!("Project inode: {project_inode}");

    let meta = fs.metadata_by_inode(project_inode).unwrap();
    println!("Project metadata: {:?}", meta.file_type);

    // --- Layer 4: FsPosix operations ---
    println!("\n--- Layer 4: FsPosix (+FsHandles, +FsLock, +FsXattr) ---\n");

    // Handle-based I/O
    let handle = fs
        .open(Path::new("/project/data.bin"), OpenFlags::WRITE)
        .unwrap();
    fs.write_at(handle, b"HEADER", 0).unwrap();
    fs.write_at(handle, b"DATA", 6).unwrap();
    fs.close(handle).unwrap();
    println!("Wrote data.bin using handle-based I/O");

    let handle = fs
        .open(Path::new("/project/data.bin"), OpenFlags::READ)
        .unwrap();
    let mut buf = [0u8; 10];
    let n = fs.read_at(handle, &mut buf, 0).unwrap();
    println!("Read {} bytes: {:?}", n, String::from_utf8_lossy(&buf[..n]));
    fs.close(handle).unwrap();

    // Locking
    let handle = fs
        .open(Path::new("/project/data.bin"), OpenFlags::READ)
        .unwrap();
    fs.lock(handle, LockType::Shared).unwrap();
    println!("Acquired shared lock");
    fs.unlock(handle).unwrap();
    println!("Released lock");
    fs.close(handle).unwrap();

    // Extended attributes
    fs.set_xattr(Path::new("/project/README.md"), "user.author", b"Alice")
        .unwrap();
    let author = fs
        .get_xattr(Path::new("/project/README.md"), "user.author")
        .unwrap();
    println!("xattr user.author = {:?}", String::from_utf8_lossy(&author));

    let attrs = fs.list_xattr(Path::new("/project/README.md")).unwrap();
    println!("All xattrs: {:?}", attrs);

    // --- Using as trait objects ---
    println!("\n--- Using as Trait Objects ---\n");

    fn use_as_fs(fs: &dyn Fs) {
        println!(
            "  Works as &dyn Fs: {} files in root",
            fs.read_dir(Path::new("/")).unwrap().count()
        );
    }

    fn use_as_full(fs: &dyn FsFull) {
        println!(
            "  Works as &dyn FsFull: {} bytes total",
            fs.statfs().unwrap().total_bytes
        );
    }

    fn use_as_fuse(fs: &dyn FsFuse) {
        println!(
            "  Works as &dyn FsFuse: root inode = {}",
            fs.path_to_inode(Path::new("/")).unwrap()
        );
    }

    fn use_as_posix(fs: &dyn FsPosix) {
        let handle = fs
            .open(Path::new("/project/README.md"), OpenFlags::READ)
            .unwrap();
        let mut buf = [0u8; 5];
        let n = fs.read_at(handle, &mut buf, 0).unwrap();
        fs.close(handle).unwrap();
        println!("  Works as &dyn FsPosix: read {} bytes", n);
    }

    use_as_fs(&fs);
    use_as_full(&fs);
    use_as_fuse(&fs);
    use_as_posix(&fs);

    println!("\n=== Reference implementation demonstration complete! ===");
}
