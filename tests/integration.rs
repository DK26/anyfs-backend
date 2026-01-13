//! Integration tests verifying the trait hierarchy works as designed.
//!
//! These tests verify that:
//! 1. The trait hierarchy composes correctly (Fs → FsFull → FsFuse → FsPosix)
//! 2. Generic functions with trait bounds work as intended
//! 3. A complete mock filesystem implementation works end-to-end
//! 4. Error handling provides useful context

use anyfs_backend::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

// =============================================================================
// Complete Mock Filesystem Implementation
// =============================================================================

/// A complete in-memory filesystem that implements ALL traits up to FsPosix.
/// This proves the trait hierarchy works as designed.
struct InMemoryFs {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
    dirs: RwLock<std::collections::HashSet<PathBuf>>,
    symlinks: RwLock<HashMap<PathBuf, PathBuf>>,
    xattrs: RwLock<HashMap<PathBuf, HashMap<String, Vec<u8>>>>,
    inodes: RwLock<HashMap<PathBuf, u64>>,
    inode_to_path: RwLock<HashMap<u64, PathBuf>>,
    handles: RwLock<HashMap<u64, OpenFile>>,
    locks: RwLock<HashMap<u64, LockState>>,
    next_inode: AtomicU64,
    next_handle: AtomicU64,
}

struct OpenFile {
    path: PathBuf,
    flags: OpenFlags,
    _position: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum LockState {
    Unlocked,
    Shared(usize),
    Exclusive,
}

impl InMemoryFs {
    fn new() -> Self {
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
        // Create root directory
        fs.dirs.write().unwrap().insert(PathBuf::from("/"));
        fs.assign_inode(Path::new("/"));
        fs
    }

    fn assign_inode(&self, path: &Path) -> u64 {
        let mut inodes = self.inodes.write().unwrap();
        if let Some(&inode) = inodes.get(path) {
            return inode;
        }
        let inode = if path == Path::new("/") {
            ROOT_INODE
        } else {
            self.next_inode.fetch_add(1, Ordering::SeqCst)
        };
        inodes.insert(path.to_path_buf(), inode);
        self.inode_to_path
            .write()
            .unwrap()
            .insert(inode, path.to_path_buf());
        inode
    }

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

// Layer 1: FsRead
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
            details: "not valid UTF-8".into(),
        })
    }

    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        let data = self.read(path)?;
        let start = offset as usize;
        let end = (start + len).min(data.len());
        if start >= data.len() {
            Ok(Vec::new())
        } else {
            Ok(data[start..end].to_vec())
        }
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

// Layer 1: FsWrite
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
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let mut files = self.files.write().unwrap();
        let data = files.remove(from).ok_or_else(|| FsError::NotFound {
            path: from.to_path_buf(),
        })?;
        files.insert(to.to_path_buf(), data);
        drop(files);
        self.assign_inode(to);
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

    fn open_write(&self, _path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        // Simple: just return a buffer that we don't actually connect
        // In a real impl this would write back on drop
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }
}

// Layer 1: FsDir
impl FsDir for InMemoryFs {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        if !self.dirs.read().unwrap().contains(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        let mut entries = Vec::new();
        let _prefix = if path == Path::new("/") {
            PathBuf::from("/")
        } else {
            path.to_path_buf()
        };

        // Collect files
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

        // Collect subdirs
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

        Ok(ReadDirIter::from_vec(entries))
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        if self.dirs.read().unwrap().contains(path) {
            return Err(FsError::AlreadyExists {
                path: path.to_path_buf(),
                operation: "create_dir",
            });
        }
        self.dirs.write().unwrap().insert(path.to_path_buf());
        self.assign_inode(path);
        Ok(())
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            if !self.dirs.read().unwrap().contains(&current) {
                self.dirs.write().unwrap().insert(current.clone());
                self.assign_inode(&current);
            }
        }
        Ok(())
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        if !self.dirs.write().unwrap().remove(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        Ok(())
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        self.dirs.write().unwrap().remove(path);
        // Also remove all children (simplified)
        self.files
            .write()
            .unwrap()
            .retain(|p, _| !p.starts_with(path));
        Ok(())
    }
}

// Layer 2: FsLink
impl FsLink for InMemoryFs {
    fn symlink(&self, target: &Path, link: &Path) -> Result<(), FsError> {
        self.symlinks
            .write()
            .unwrap()
            .insert(link.to_path_buf(), target.to_path_buf());
        self.assign_inode(link);
        Ok(())
    }

    fn hard_link(&self, original: &Path, link: &Path) -> Result<(), FsError> {
        let data = self.read(original)?;
        self.write(link, &data)?;
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
                details: "not a symlink".into(),
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

// Layer 2: FsPermissions
impl FsPermissions for InMemoryFs {
    fn set_permissions(&self, path: &Path, _perm: Permissions) -> Result<(), FsError> {
        if self.get_file_type(path).is_none() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        Ok(())
    }
}

// Layer 2: FsSync
impl FsSync for InMemoryFs {
    fn sync(&self) -> Result<(), FsError> {
        Ok(())
    }

    fn fsync(&self, path: &Path) -> Result<(), FsError> {
        if self.get_file_type(path).is_none() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        Ok(())
    }
}

// Layer 2: FsStats
impl FsStats for InMemoryFs {
    fn statfs(&self) -> Result<StatFs, FsError> {
        Ok(StatFs {
            total_bytes: 1024 * 1024 * 100, // 100MB
            used_bytes: 1024 * 1024 * 50,
            available_bytes: 1024 * 1024 * 50,
            total_inodes: 10000,
            used_inodes: 1000,
            available_inodes: 9000,
            block_size: 4096,
            max_name_len: 255,
        })
    }
}

// Layer 3: FsInode
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

// Layer 4: FsHandles
impl FsHandles for InMemoryFs {
    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Handle, FsError> {
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
                _position: 0,
            },
        );
        Ok(Handle(handle_id))
    }

    fn read_at(&self, handle: Handle, buf: &mut [u8], offset: u64) -> Result<usize, FsError> {
        let handles = self.handles.read().unwrap();
        let open_file = handles
            .get(&handle.0)
            .ok_or(FsError::InvalidHandle { handle })?;

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
        let handles = self.handles.read().unwrap();
        let open_file = handles
            .get(&handle.0)
            .ok_or(FsError::InvalidHandle { handle })?;

        if !open_file.flags.write {
            return Err(FsError::PermissionDenied {
                path: open_file.path.clone(),
                operation: "write",
            });
        }

        let path = open_file.path.clone();
        drop(handles);

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
        self.handles
            .write()
            .unwrap()
            .remove(&handle.0)
            .map(|_| ())
            .ok_or(FsError::InvalidHandle { handle })
    }
}

// Layer 4: FsLock
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

// Layer 4: FsXattr
impl FsXattr for InMemoryFs {
    fn get_xattr(&self, path: &Path, name: &str) -> Result<Vec<u8>, FsError> {
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
// Tests: Trait Hierarchy Verification
// =============================================================================

/// Verify InMemoryFs can be used as `&dyn Fs` (Layer 1)
#[test]
fn trait_hierarchy_fs_works() {
    let fs = InMemoryFs::new();
    verify_fs_trait(&fs);
}

fn verify_fs_trait(fs: &dyn Fs) {
    // Write a file
    fs.write(Path::new("/test.txt"), b"hello").unwrap();

    // Read it back
    let data = fs.read(Path::new("/test.txt")).unwrap();
    assert_eq!(data, b"hello");

    // Check it exists
    assert!(fs.exists(Path::new("/test.txt")).unwrap());

    // Create a directory
    fs.create_dir(Path::new("/subdir")).unwrap();

    // List root directory
    let entries: Vec<_> = fs
        .read_dir(Path::new("/"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(entries.len(), 2); // test.txt and subdir
}

/// Verify InMemoryFs can be used as `&dyn FsFull` (Layer 2)
#[test]
fn trait_hierarchy_fs_full_works() {
    let fs = InMemoryFs::new();
    verify_fs_full_trait(&fs);
}

fn verify_fs_full_trait(fs: &dyn FsFull) {
    // Create a file
    fs.write(Path::new("/original.txt"), b"data").unwrap();

    // Create a symlink
    fs.symlink(Path::new("/original.txt"), Path::new("/link.txt"))
        .unwrap();

    // Read the symlink target
    let target = fs.read_link(Path::new("/link.txt")).unwrap();
    assert_eq!(target, PathBuf::from("/original.txt"));

    // Get stats
    let stats = fs.statfs().unwrap();
    assert!(stats.total_bytes > 0);

    // Sync
    fs.sync().unwrap();
}

/// Verify InMemoryFs can be used as `&dyn FsFuse` (Layer 3)
#[test]
fn trait_hierarchy_fs_fuse_works() {
    let fs = InMemoryFs::new();
    verify_fs_fuse_trait(&fs);
}

fn verify_fs_fuse_trait(fs: &dyn FsFuse) {
    // Create a file
    fs.write(Path::new("/file.txt"), b"content").unwrap();

    // Get root inode
    let root_inode = fs.path_to_inode(Path::new("/")).unwrap();
    assert_eq!(root_inode, ROOT_INODE);

    // Lookup file by name from root
    let file_inode = fs.lookup(root_inode, OsStr::new("file.txt")).unwrap();
    assert!(file_inode > ROOT_INODE);

    // Get metadata by inode
    let meta = fs.metadata_by_inode(file_inode).unwrap();
    assert_eq!(meta.file_type, FileType::File);
    assert_eq!(meta.size, 7); // "content".len()

    // Round-trip: inode -> path -> inode
    let path = fs.inode_to_path(file_inode).unwrap();
    let inode2 = fs.path_to_inode(&path).unwrap();
    assert_eq!(file_inode, inode2);
}

/// Verify InMemoryFs can be used as `&dyn FsPosix` (Layer 4)
#[test]
fn trait_hierarchy_fs_posix_works() {
    let fs = InMemoryFs::new();
    verify_fs_posix_trait(&fs);
}

fn verify_fs_posix_trait(fs: &dyn FsPosix) {
    // Create and open a file
    let handle = fs.open(Path::new("/posix.txt"), OpenFlags::WRITE).unwrap();

    // Write via handle
    let written = fs.write_at(handle, b"hello posix", 0).unwrap();
    assert_eq!(written, 11);

    // Lock the file
    fs.lock(handle, LockType::Exclusive).unwrap();

    // Unlock and close
    fs.unlock(handle).unwrap();
    fs.close(handle).unwrap();

    // Set xattr
    fs.set_xattr(Path::new("/posix.txt"), "user.test", b"value")
        .unwrap();

    // Get xattr
    let value = fs.get_xattr(Path::new("/posix.txt"), "user.test").unwrap();
    assert_eq!(value, b"value");

    // List xattrs
    let attrs = fs.list_xattr(Path::new("/posix.txt")).unwrap();
    assert_eq!(attrs, vec!["user.test"]);
}

// =============================================================================
// Tests: Generic Function Trait Bounds (from Design Manual)
// =============================================================================

/// Test: Generic function requiring only `Fs` works
#[test]
fn generic_function_with_fs_bound() {
    fn process_files<B: Fs>(fs: &B) -> Result<(), FsError> {
        fs.write(Path::new("/input.txt"), b"input data")?;
        let data = fs.read(Path::new("/input.txt"))?;
        fs.write(Path::new("/output.txt"), &data)?;
        Ok(())
    }

    let fs = InMemoryFs::new();
    process_files(&fs).unwrap();

    assert_eq!(fs.read(Path::new("/output.txt")).unwrap(), b"input data");
}

/// Test: Generic function requiring `Fs + FsLink` works
#[test]
fn generic_function_with_link_bound() {
    fn create_backup<B: Fs + FsLink>(fs: &B) -> Result<(), FsError> {
        fs.write(Path::new("/data.txt"), b"important")?;
        fs.hard_link(Path::new("/data.txt"), Path::new("/data.txt.bak"))?;
        Ok(())
    }

    let fs = InMemoryFs::new();
    create_backup(&fs).unwrap();

    // Both files should have the same content
    assert_eq!(
        fs.read(Path::new("/data.txt")).unwrap(),
        fs.read(Path::new("/data.txt.bak")).unwrap()
    );
}

/// Test: Generic function requiring `FsFuse` works
#[test]
fn generic_function_with_fuse_bound() {
    fn get_file_inode<B: FsFuse>(fs: &B, name: &str) -> Result<u64, FsError> {
        let root = fs.path_to_inode(Path::new("/"))?;
        fs.lookup(root, OsStr::new(name))
    }

    let fs = InMemoryFs::new();
    fs.write(Path::new("/myfile.txt"), b"test").unwrap();

    let inode = get_file_inode(&fs, "myfile.txt").unwrap();
    assert!(inode > ROOT_INODE);
}

/// Test: Generic function requiring `FsPosix` works
#[test]
fn generic_function_with_posix_bound() {
    fn atomic_write<B: FsPosix>(fs: &B, path: &Path, data: &[u8]) -> Result<(), FsError> {
        let handle = fs.open(path, OpenFlags::WRITE)?;
        fs.lock(handle, LockType::Exclusive)?;
        fs.write_at(handle, data, 0)?;
        fs.unlock(handle)?;
        fs.close(handle)?;
        Ok(())
    }

    let fs = InMemoryFs::new();
    atomic_write(&fs, Path::new("/atomic.txt"), b"atomic data").unwrap();

    assert_eq!(fs.read(Path::new("/atomic.txt")).unwrap(), b"atomic data");
}

// =============================================================================
// Tests: Error Handling Verification
// =============================================================================

#[test]
fn error_not_found_contains_path() {
    let fs = InMemoryFs::new();
    let result = fs.read(Path::new("/nonexistent.txt"));

    match result {
        Err(FsError::NotFound { path }) => {
            assert_eq!(path, PathBuf::from("/nonexistent.txt"));
        }
        _ => panic!("expected NotFound error"),
    }
}

#[test]
fn error_already_exists_contains_context() {
    let fs = InMemoryFs::new();
    fs.create_dir(Path::new("/mydir")).unwrap();
    let result = fs.create_dir(Path::new("/mydir"));

    match result {
        Err(FsError::AlreadyExists { path, operation }) => {
            assert_eq!(path, PathBuf::from("/mydir"));
            assert_eq!(operation, "create_dir");
        }
        _ => panic!("expected AlreadyExists error"),
    }
}

#[test]
fn error_invalid_handle_contains_handle() {
    let fs = InMemoryFs::new();
    let bogus_handle = Handle(9999);
    let result = fs.close(bogus_handle);

    match result {
        Err(FsError::InvalidHandle { handle }) => {
            assert_eq!(handle.0, 9999);
        }
        _ => panic!("expected InvalidHandle error"),
    }
}

#[test]
fn error_xattr_not_found_contains_name() {
    let fs = InMemoryFs::new();
    fs.write(Path::new("/file.txt"), b"data").unwrap();
    let result = fs.get_xattr(Path::new("/file.txt"), "user.missing");

    match result {
        Err(FsError::XattrNotFound { path, name }) => {
            assert_eq!(path, PathBuf::from("/file.txt"));
            assert_eq!(name, "user.missing");
        }
        _ => panic!("expected XattrNotFound error"),
    }
}

#[test]
fn error_inode_not_found() {
    let fs = InMemoryFs::new();
    let result = fs.inode_to_path(99999);

    match result {
        Err(FsError::InodeNotFound { inode }) => {
            assert_eq!(inode, 99999);
        }
        _ => panic!("expected InodeNotFound error"),
    }
}

// =============================================================================
// Tests: FsPath Blanket Implementation
// =============================================================================

#[test]
fn fs_path_canonicalize_works() {
    let fs = InMemoryFs::new();
    fs.create_dir(Path::new("/a")).unwrap();
    fs.create_dir(Path::new("/a/b")).unwrap();
    fs.write(Path::new("/a/b/file.txt"), b"data").unwrap();

    // FsPath is automatically implemented because InMemoryFs: FsRead + FsLink
    let canonical = fs.canonicalize(Path::new("/a/b/../b/./file.txt")).unwrap();
    assert_eq!(canonical, PathBuf::from("/a/b/file.txt"));
}

#[test]
fn fs_path_follows_symlinks() {
    let fs = InMemoryFs::new();
    fs.create_dir(Path::new("/real")).unwrap();
    fs.write(Path::new("/real/file.txt"), b"data").unwrap();
    fs.symlink(Path::new("/real"), Path::new("/link")).unwrap();

    // Canonicalize should follow the symlink
    let canonical = fs.canonicalize(Path::new("/link/file.txt")).unwrap();
    assert_eq!(canonical, PathBuf::from("/real/file.txt"));
}

#[test]
fn fs_path_soft_canonicalize_allows_nonexistent() {
    let fs = InMemoryFs::new();
    fs.create_dir(Path::new("/existing")).unwrap();

    // soft_canonicalize allows the final component to not exist
    let result = fs.soft_canonicalize(Path::new("/existing/new_file.txt"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), PathBuf::from("/existing/new_file.txt"));
}

// =============================================================================
// Tests: Real Workflows
// =============================================================================

#[test]
fn workflow_create_project_structure() {
    let fs = InMemoryFs::new();

    // Create a project structure
    fs.create_dir_all(Path::new("/project/src")).unwrap();
    fs.create_dir_all(Path::new("/project/tests")).unwrap();

    fs.write(
        Path::new("/project/Cargo.toml"),
        b"[package]\nname = \"myproject\"",
    )
    .unwrap();
    fs.write(Path::new("/project/src/main.rs"), b"fn main() {}")
        .unwrap();
    fs.write(
        Path::new("/project/tests/test.rs"),
        b"#[test]\nfn it_works() {}",
    )
    .unwrap();

    // Verify structure
    assert!(fs.exists(Path::new("/project/src/main.rs")).unwrap());
    assert!(fs.exists(Path::new("/project/Cargo.toml")).unwrap());

    let src_entries: Vec<_> = fs
        .read_dir(Path::new("/project/src"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(src_entries.len(), 1);
    assert_eq!(src_entries[0].name, "main.rs");
}

#[test]
fn workflow_copy_and_rename() {
    let fs = InMemoryFs::new();

    fs.write(Path::new("/original.txt"), b"original content")
        .unwrap();

    // Copy the file
    fs.copy(Path::new("/original.txt"), Path::new("/copy.txt"))
        .unwrap();

    // Rename the copy
    fs.rename(Path::new("/copy.txt"), Path::new("/renamed.txt"))
        .unwrap();

    // Verify
    assert!(fs.exists(Path::new("/original.txt")).unwrap());
    assert!(!fs.exists(Path::new("/copy.txt")).unwrap());
    assert!(fs.exists(Path::new("/renamed.txt")).unwrap());
    assert_eq!(
        fs.read(Path::new("/renamed.txt")).unwrap(),
        b"original content"
    );
}

#[test]
fn workflow_handle_based_read_write() {
    let fs = InMemoryFs::new();

    // Open for write
    let write_handle = fs.open(Path::new("/data.bin"), OpenFlags::WRITE).unwrap();
    fs.write_at(write_handle, b"HEADER", 0).unwrap();
    fs.write_at(write_handle, b"BODY", 6).unwrap();
    fs.close(write_handle).unwrap();

    // Open for read
    let read_handle = fs.open(Path::new("/data.bin"), OpenFlags::READ).unwrap();
    let mut buf = [0u8; 10];
    let n = fs.read_at(read_handle, &mut buf, 0).unwrap();
    fs.close(read_handle).unwrap();

    assert_eq!(n, 10);
    assert_eq!(&buf, b"HEADERBODY");
}

#[test]
fn workflow_metadata_and_stats() {
    let fs = InMemoryFs::new();

    fs.write(Path::new("/file.txt"), b"12345").unwrap();
    fs.create_dir(Path::new("/dir")).unwrap();
    fs.symlink(Path::new("/file.txt"), Path::new("/link"))
        .unwrap();

    // Check file metadata
    let file_meta = fs.metadata(Path::new("/file.txt")).unwrap();
    assert_eq!(file_meta.file_type, FileType::File);
    assert_eq!(file_meta.size, 5);

    // Check directory metadata
    let dir_meta = fs.metadata(Path::new("/dir")).unwrap();
    assert_eq!(dir_meta.file_type, FileType::Directory);

    // Check symlink metadata (without following)
    let link_meta = fs.symlink_metadata(Path::new("/link")).unwrap();
    assert_eq!(link_meta.file_type, FileType::Symlink);

    // Check filesystem stats
    let stats = fs.statfs().unwrap();
    assert!(stats.total_bytes > stats.used_bytes);
}
