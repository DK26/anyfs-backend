//! Writing generic functions with anyfs-backend trait bounds.
//!
//! This example demonstrates how to write reusable code that works
//! with ANY filesystem backend by using trait bounds.
//!
//! Run with: `cargo run --example generic_functions`

use anyfs_backend::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::SystemTime;

// =============================================================================
// Pattern 1: Generic functions with `Fs` bound (most common)
// =============================================================================

/// Copy a file's contents, works with ANY backend implementing `Fs`.
///
/// This is the most common pattern - just require `Fs` for basic operations.
fn copy_file<B: Fs>(fs: &B, from: &Path, to: &Path) -> Result<(), FsError> {
    let data = fs.read(from)?;
    fs.write(to, &data)?;
    Ok(())
}

/// Count all files recursively in a directory.
fn count_files_recursive<B: Fs>(fs: &B, dir: &Path) -> Result<usize, FsError> {
    let mut count = 0;
    for entry in fs.read_dir(dir)? {
        let entry = entry?;
        match entry.file_type {
            FileType::File => count += 1,
            FileType::Directory => {
                count += count_files_recursive(fs, &entry.path)?;
            }
            FileType::Symlink => {} // Skip symlinks in basic count
        }
    }
    Ok(count)
}

/// Find files matching a predicate.
fn find_files<B: Fs>(
    fs: &B,
    dir: &Path,
    predicate: &dyn Fn(&DirEntry) -> bool,
) -> Result<Vec<PathBuf>, FsError> {
    let mut results = Vec::new();
    for entry in fs.read_dir(dir)? {
        let entry = entry?;
        if predicate(&entry) {
            results.push(entry.path.clone());
        }
        if entry.file_type == FileType::Directory {
            results.extend(find_files(fs, &entry.path, predicate)?);
        }
    }
    Ok(results)
}

// =============================================================================
// Pattern 2: Multiple trait bounds (Fs + FsLink)
// =============================================================================

/// Create a backup with a symlink to the original.
///
/// Requires both `Fs` (for read/write) and `FsLink` (for symlinks).
fn backup_with_link<B: Fs + FsLink>(fs: &B, path: &Path) -> Result<PathBuf, FsError> {
    let backup_path = PathBuf::from(format!("{}.bak", path.display()));

    // Copy the file
    let data = fs.read(path)?;
    fs.write(&backup_path, &data)?;

    // Create a symlink pointing to the backup
    let link_path = PathBuf::from(format!("{}.latest", path.display()));
    // Remove old link if exists (ignore errors)
    let _ = fs.remove_file(&link_path);
    fs.symlink(&backup_path, &link_path)?;

    Ok(backup_path)
}

/// Resolve all symlinks in a path and return the real path.
fn resolve_symlinks<B: Fs + FsLink>(fs: &B, path: &Path) -> Result<PathBuf, FsError> {
    let mut current = path.to_path_buf();
    let mut seen = std::collections::HashSet::new();

    loop {
        if !seen.insert(current.clone()) {
            // Detected a symlink loop
            return Err(FsError::InvalidData {
                path: current,
                details: "symlink loop detected".into(),
            });
        }

        match fs.symlink_metadata(&current) {
            Ok(meta) if meta.file_type == FileType::Symlink => {
                let target = fs.read_link(&current)?;
                current = if target.is_absolute() {
                    target
                } else {
                    current.parent().unwrap_or(Path::new("/")).join(target)
                };
            }
            Ok(_) => return Ok(current),
            Err(e) => return Err(e),
        }
    }
}

// =============================================================================
// Pattern 3: Using composite traits (FsFull, FsFuse, FsPosix)
// =============================================================================

/// Get filesystem statistics and report usage.
///
/// Requires `FsFull` which includes `FsStats`.
fn report_usage<B: FsFull>(fs: &B) -> Result<String, FsError> {
    let stats = fs.statfs()?;
    let used_percent = (stats.used_bytes as f64 / stats.total_bytes as f64) * 100.0;

    Ok(format!(
        "Disk usage: {:.1}% ({} / {} bytes)",
        used_percent, stats.used_bytes, stats.total_bytes
    ))
}

/// Navigate filesystem by inode (FUSE-style).
///
/// Requires `FsFuse` which includes `FsInode`.
fn list_by_inode<B: FsFuse>(fs: &B, inode: u64) -> Result<Vec<(String, u64)>, FsError> {
    let path = fs.inode_to_path(inode)?;
    let mut entries = Vec::new();

    for entry in fs.read_dir(&path)? {
        let entry = entry?;
        let child_inode = fs.path_to_inode(&entry.path)?;
        entries.push((entry.name, child_inode));
    }

    Ok(entries)
}

/// Perform atomic file write with locking.
///
/// Requires `FsPosix` which includes `FsHandles` and `FsLock`.
fn atomic_write<B: FsPosix>(fs: &B, path: &Path, data: &[u8]) -> Result<(), FsError> {
    // Open with write flag
    let handle = fs.open(path, OpenFlags::WRITE)?;

    // Lock exclusively
    fs.lock(handle, LockType::Exclusive)?;

    // Write data
    fs.write_at(handle, data, 0)?;

    // Unlock and close
    fs.unlock(handle)?;
    fs.close(handle)?;

    Ok(())
}

// =============================================================================
// Pattern 4: Trait objects for runtime polymorphism
// =============================================================================

/// Process files using a trait object.
///
/// Useful when the backend type isn't known at compile time.
fn process_with_trait_object(fs: &dyn Fs, files: &[&Path]) -> Result<u64, FsError> {
    let mut total_size = 0;
    for path in files {
        let meta = fs.metadata(path)?;
        total_size += meta.size;
    }
    Ok(total_size)
}

/// Store multiple backends with different types.
struct MultiBackend {
    backends: Vec<Box<dyn Fs>>,
}

impl MultiBackend {
    fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    fn add<B: Fs + 'static>(&mut self, backend: B) {
        self.backends.push(Box::new(backend));
    }

    fn find_file(&self, name: &str) -> Option<(usize, PathBuf)> {
        for (i, fs) in self.backends.iter().enumerate() {
            if let Ok(entries) = fs.read_dir(Path::new("/")) {
                for entry in entries.flatten() {
                    if entry.name == name {
                        return Some((i, entry.path));
                    }
                }
            }
        }
        None
    }
}

// =============================================================================
// Pattern 5: Extension traits for custom functionality
// =============================================================================

// Note: This pattern is shown in the crate's FsExtJson trait.
// To use it, enable the `serde` feature and use the FsExtJson extension trait.
//
// Example (with serde feature):
// ```
// use anyfs_backend::ext::FsExtJson;
// let config: MyConfig = fs.read_json(Path::new("/config.json"))?;
// fs.write_json(Path::new("/output.json"), &my_data)?;
// ```

// =============================================================================
// Demo implementation (same as basic_usage.rs, but more complete)
// =============================================================================

struct DemoFs {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
    dirs: RwLock<std::collections::HashSet<PathBuf>>,
    symlinks: RwLock<HashMap<PathBuf, PathBuf>>,
    inodes: RwLock<HashMap<PathBuf, u64>>,
    inode_to_path: RwLock<HashMap<u64, PathBuf>>,
    handles: RwLock<HashMap<u64, (PathBuf, OpenFlags)>>,
    next_inode: AtomicU64,
    next_handle: AtomicU64,
}

impl DemoFs {
    fn new() -> Self {
        let fs = Self {
            files: RwLock::new(HashMap::new()),
            dirs: RwLock::new(std::collections::HashSet::new()),
            symlinks: RwLock::new(HashMap::new()),
            inodes: RwLock::new(HashMap::new()),
            inode_to_path: RwLock::new(HashMap::new()),
            handles: RwLock::new(HashMap::new()),
            next_inode: AtomicU64::new(2),
            next_handle: AtomicU64::new(1),
        };
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

// Implement all required traits (abbreviated for clarity)
impl FsRead for DemoFs {
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
        String::from_utf8(self.read(path)?).map_err(|_| FsError::InvalidData {
            path: path.to_path_buf(),
            details: "not UTF-8".into(),
        })
    }
    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        let data = self.read(path)?;
        let start = offset as usize;
        Ok(if start >= data.len() {
            vec![]
        } else {
            data[start..(start + len).min(data.len())].to_vec()
        })
    }
    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        Ok(self.get_file_type(path).is_some())
    }
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let ft = self.get_file_type(path).ok_or_else(|| FsError::NotFound {
            path: path.to_path_buf(),
        })?;
        let size = if ft == FileType::File {
            self.files
                .read()
                .unwrap()
                .get(path)
                .map(|d| d.len() as u64)
                .unwrap_or(0)
        } else {
            0
        };
        Ok(Metadata {
            file_type: ft,
            size,
            permissions: Permissions::default_file(),
            created: SystemTime::UNIX_EPOCH,
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            inode: self.inodes.read().unwrap().get(path).copied().unwrap_or(0),
            nlink: 1,
        })
    }
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
        Ok(Box::new(std::io::Cursor::new(self.read(path)?)))
    }
}

impl FsWrite for DemoFs {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.assign_inode(path);
        self.files
            .write()
            .unwrap()
            .insert(path.to_path_buf(), data.to_vec());
        Ok(())
    }
    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .entry(path.to_path_buf())
            .or_default()
            .extend_from_slice(data);
        self.assign_inode(path);
        Ok(())
    }
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .remove(path)
            .map(|_| ())
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let data = self
            .files
            .write()
            .unwrap()
            .remove(from)
            .ok_or_else(|| FsError::NotFound {
                path: from.to_path_buf(),
            })?;
        self.files.write().unwrap().insert(to.to_path_buf(), data);
        self.assign_inode(to);
        Ok(())
    }
    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let data = self.read(from)?;
        self.write(to, &data)
    }
    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .get_mut(path)
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })?
            .resize(size as usize, 0);
        Ok(())
    }
    fn open_write(&self, _path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }
}

impl FsDir for DemoFs {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        if !self.dirs.read().unwrap().contains(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        let mut entries = Vec::new();
        for (fp, data) in self.files.read().unwrap().iter() {
            if fp.parent() == Some(path) {
                if let Some(name) = fp.file_name() {
                    entries.push(Ok(DirEntry {
                        name: name.to_string_lossy().into(),
                        path: fp.clone(),
                        file_type: FileType::File,
                        size: data.len() as u64,
                        inode: 0,
                    }));
                }
            }
        }
        for dp in self.dirs.read().unwrap().iter() {
            if dp.parent() == Some(path) && dp != path {
                if let Some(name) = dp.file_name() {
                    entries.push(Ok(DirEntry {
                        name: name.to_string_lossy().into(),
                        path: dp.clone(),
                        file_type: FileType::Directory,
                        size: 0,
                        inode: 0,
                    }));
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
        for c in path.components() {
            current.push(c);
            self.dirs.write().unwrap().insert(current.clone());
            self.assign_inode(&current);
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
        self.files
            .write()
            .unwrap()
            .retain(|p, _| !p.starts_with(path));
        Ok(())
    }
}

impl FsLink for DemoFs {
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
        self.write(link, &data)
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
        let ft = self.get_file_type(path).ok_or_else(|| FsError::NotFound {
            path: path.to_path_buf(),
        })?;
        Ok(Metadata {
            file_type: ft,
            size: 0,
            permissions: Permissions::default_file(),
            created: SystemTime::UNIX_EPOCH,
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            inode: 0,
            nlink: 1,
        })
    }
}

impl FsPermissions for DemoFs {
    fn set_permissions(&self, path: &Path, _perm: Permissions) -> Result<(), FsError> {
        if self.get_file_type(path).is_none() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        Ok(())
    }
}

impl FsSync for DemoFs {
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

impl FsStats for DemoFs {
    fn statfs(&self) -> Result<StatFs, FsError> {
        Ok(StatFs {
            total_bytes: 100 * 1024 * 1024,
            used_bytes: 50 * 1024 * 1024,
            available_bytes: 50 * 1024 * 1024,
            total_inodes: 10000,
            used_inodes: 1000,
            available_inodes: 9000,
            block_size: 4096,
            max_name_len: 255,
        })
    }
}

impl FsInode for DemoFs {
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
    fn lookup(&self, parent: u64, name: &OsStr) -> Result<u64, FsError> {
        let parent_path = self.inode_to_path(parent)?;
        self.path_to_inode(&parent_path.join(name))
    }
    fn metadata_by_inode(&self, inode: u64) -> Result<Metadata, FsError> {
        self.metadata(&self.inode_to_path(inode)?)
    }
}

impl FsHandles for DemoFs {
    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Handle, FsError> {
        if flags.create && !self.files.read().unwrap().contains_key(path) {
            self.write(path, &[])?;
        } else if !self.files.read().unwrap().contains_key(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        let id = self.next_handle.fetch_add(1, Ordering::SeqCst);
        self.handles
            .write()
            .unwrap()
            .insert(id, (path.to_path_buf(), flags));
        Ok(Handle(id))
    }
    fn read_at(&self, handle: Handle, buf: &mut [u8], offset: u64) -> Result<usize, FsError> {
        let handles = self.handles.read().unwrap();
        let (path, flags) = handles
            .get(&handle.0)
            .ok_or(FsError::InvalidHandle { handle })?;
        if !flags.read {
            return Err(FsError::PermissionDenied {
                path: path.clone(),
                operation: "read",
            });
        }
        let data = self
            .files
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .unwrap_or_default();
        let start = offset as usize;
        if start >= data.len() {
            return Ok(0);
        }
        let n = buf.len().min(data.len() - start);
        buf[..n].copy_from_slice(&data[start..start + n]);
        Ok(n)
    }
    fn write_at(&self, handle: Handle, data: &[u8], offset: u64) -> Result<usize, FsError> {
        let handles = self.handles.read().unwrap();
        let (path, flags) = handles
            .get(&handle.0)
            .ok_or(FsError::InvalidHandle { handle })?;
        if !flags.write {
            return Err(FsError::PermissionDenied {
                path: path.clone(),
                operation: "write",
            });
        }
        let path = path.clone();
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

impl FsLock for DemoFs {
    fn lock(&self, handle: Handle, _lock_type: LockType) -> Result<(), FsError> {
        if !self.handles.read().unwrap().contains_key(&handle.0) {
            return Err(FsError::InvalidHandle { handle });
        }
        Ok(())
    }
    fn try_lock(&self, handle: Handle, _lock_type: LockType) -> Result<bool, FsError> {
        if !self.handles.read().unwrap().contains_key(&handle.0) {
            return Err(FsError::InvalidHandle { handle });
        }
        Ok(true)
    }
    fn unlock(&self, handle: Handle) -> Result<(), FsError> {
        if !self.handles.read().unwrap().contains_key(&handle.0) {
            return Err(FsError::InvalidHandle { handle });
        }
        Ok(())
    }
}

impl FsXattr for DemoFs {
    fn get_xattr(&self, path: &Path, _name: &str) -> Result<Vec<u8>, FsError> {
        Err(FsError::NotFound {
            path: path.to_path_buf(),
        })
    }
    fn set_xattr(&self, _path: &Path, _name: &str, _value: &[u8]) -> Result<(), FsError> {
        Ok(())
    }
    fn remove_xattr(&self, path: &Path, name: &str) -> Result<(), FsError> {
        Err(FsError::XattrNotFound {
            path: path.to_path_buf(),
            name: name.to_string(),
        })
    }
    fn list_xattr(&self, _path: &Path) -> Result<Vec<String>, FsError> {
        Ok(vec![])
    }
}

// =============================================================================
// Main: Demonstrate all patterns
// =============================================================================

fn main() {
    println!("=== Generic Functions Example ===\n");

    let fs = DemoFs::new();

    // Setup: Create some test files
    fs.create_dir_all(Path::new("/project/src")).unwrap();
    fs.write(Path::new("/project/README.md"), b"# My Project")
        .unwrap();
    fs.write(Path::new("/project/src/main.rs"), b"fn main() {}")
        .unwrap();
    fs.write(Path::new("/project/src/lib.rs"), b"pub fn hello() {}")
        .unwrap();

    // Pattern 1: Basic Fs operations
    println!("Pattern 1: Functions with Fs bound");
    copy_file(
        &fs,
        Path::new("/project/README.md"),
        Path::new("/project/README.bak"),
    )
    .unwrap();
    println!("  Copied README.md to README.bak");

    let count = count_files_recursive(&fs, Path::new("/project")).unwrap();
    println!("  Total files in /project: {count}");

    let rs_files = find_files(&fs, Path::new("/project"), &|e| e.name.ends_with(".rs")).unwrap();
    println!("  Rust files found: {rs_files:?}");

    // Pattern 2: Fs + FsLink
    println!("\nPattern 2: Functions with Fs + FsLink bounds");
    let backup = backup_with_link(&fs, Path::new("/project/README.md")).unwrap();
    println!("  Created backup at: {}", backup.display());

    // Also demonstrate symlink resolution (the .latest link was created above)
    let resolved = resolve_symlinks(&fs, Path::new("/project/README.md.latest")).unwrap();
    println!("  Resolved symlink to: {}", resolved.display());

    // Pattern 3: FsFull (includes FsStats)
    println!("\nPattern 3: Functions with FsFull bound");
    let usage = report_usage(&fs).unwrap();
    println!("  {usage}");

    // Pattern 3: FsFuse (includes FsInode)
    println!("\nPattern 4: Functions with FsFuse bound");
    let entries = list_by_inode(&fs, ROOT_INODE).unwrap();
    println!("  Root directory contents by inode:");
    for (name, inode) in entries {
        println!("    {name}: inode {inode}");
    }

    // Pattern 3: FsPosix (includes FsHandles + FsLock)
    println!("\nPattern 5: Functions with FsPosix bound");
    atomic_write(&fs, Path::new("/project/config.txt"), b"key=value").unwrap();
    println!("  Wrote config.txt atomically with locking");

    // Pattern 4: Trait objects
    println!("\nPattern 6: Trait objects for runtime polymorphism");
    let files = [
        Path::new("/project/README.md"),
        Path::new("/project/src/main.rs"),
    ];
    let total = process_with_trait_object(&fs, &files).unwrap();
    println!("  Total size of selected files: {total} bytes");

    // Pattern 4b: Storing multiple backends
    let mut multi = MultiBackend::new();
    multi.add(DemoFs::new()); // Could add different backend types here
    println!("  MultiBackend can store {} backends", multi.backends.len());
    if let Some((idx, path)) = multi.find_file("README.md") {
        println!("  Found file in backend {idx}: {}", path.display());
    }

    // Pattern 5: Extension traits
    // With the `serde` feature enabled, you can use FsExtJson for JSON operations.
    // See the crate documentation for FsExtJson examples.

    println!("\n=== All patterns demonstrated! ===");
}
