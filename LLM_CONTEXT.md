# anyfs-backend

Trait definitions for pluggable virtual filesystems. Version 0.1.0-pre.1, MSRV 1.68.

## Fs Trait - Basic Filesystem (90% of use cases)

```rust
use anyfs_backend::{Fs, FsError};
use std::path::Path;

fn work_with_files<B: Fs>(fs: &B) -> Result<(), FsError> {
    // Read file
    let data: Vec<u8> = fs.read(Path::new("/file.txt"))?;
    
    // Read as string
    let text: String = fs.read_to_string(Path::new("/file.txt"))?;
    
    // Read range (offset, length)
    let chunk: Vec<u8> = fs.read_range(Path::new("/file.txt"), 0, 1024)?;
    
    // Write file (creates or overwrites)
    fs.write(Path::new("/output.txt"), b"Hello")?;
    
    // Append to file
    fs.append(Path::new("/log.txt"), b"Entry\n")?;
    
    // Check existence
    if fs.exists(Path::new("/file.txt"))? {
        let meta = fs.metadata(Path::new("/file.txt"))?;
        println!("Size: {}", meta.size);
    }
    
    // Create directory
    fs.create_dir(Path::new("/data"))?;
    
    // Create with parents
    fs.create_dir_all(Path::new("/data/archive/2024"))?;
    
    // List directory
    for entry in fs.read_dir(Path::new("/data"))? {
        let entry = entry?;
        println!("{}: {:?}", entry.name, entry.file_type);
    }
    
    // Remove file
    fs.remove_file(Path::new("/old.txt"))?;
    
    // Remove directory
    fs.remove_dir(Path::new("/empty"))?;
    fs.remove_dir_all(Path::new("/data"))?;
    
    // Rename/move
    fs.rename(Path::new("/old"), Path::new("/new"))?;
    
    // Copy
    fs.copy(Path::new("/src.txt"), Path::new("/dst.txt"))?;
    
    Ok(())
}
```

## FsFull Trait - Extended Features

```rust
use anyfs_backend::{FsFull, FsError, Permissions};
use std::path::Path;

fn extended_ops<B: FsFull>(fs: &B) -> Result<(), FsError> {
    // Symbolic link
    fs.symlink(Path::new("/target"), Path::new("/link"))?;
    
    // Hard link
    fs.hard_link(Path::new("/file"), Path::new("/hardlink"))?;
    
    // Read link target
    let target = fs.read_link(Path::new("/link"))?;
    
    // Set permissions
    fs.set_permissions(Path::new("/script.sh"), Permissions::from_mode(0o755))?;
    
    // Sync to disk
    fs.sync()?;
    
    // Filesystem stats
    let stats = fs.statfs()?;
    println!("Total: {} Free: {}", stats.total_bytes, stats.free_bytes);
    
    Ok(())
}
```

## Trait Hierarchy

```
Fs       = FsRead + FsWrite + FsDir
FsFull   = Fs + FsLink + FsPermissions + FsSync + FsStats
FsFuse   = FsFull + FsInode
FsPosix  = FsFuse + FsHandles + FsLock + FsXattr
```

## Complete Trait Reference

### Component Traits

| Trait           | Provides              | Key Methods                                | When to Use                |
| --------------- | --------------------- | ------------------------------------------ | -------------------------- |
| `FsRead`        | Read operations       | `read`, `exists`, `metadata`, `open_read`  | Always (core)              |
| `FsWrite`       | Write operations      | `write`, `append`, `remove_file`, `rename` | Always (core)              |
| `FsDir`         | Directory operations  | `read_dir`, `create_dir`, `remove_dir_all` | Always (core)              |
| `FsLink`        | Symbolic/hard links   | `symlink`, `hard_link`, `read_link`        | Backup tools               |
| `FsPermissions` | Permission management | `set_permissions`                          | File managers              |
| `FsSync`        | Force disk sync       | `sync`, `fsync`                            | Databases                  |
| `FsStats`       | Filesystem statistics | `statfs`                                   | Disk monitoring            |
| `FsInode`       | Inode ↔ path mapping  | `path_to_inode`, `inode_to_path`, `lookup` | FUSE filesystems           |
| `FsHandles`     | Open file handles     | `open`, `close`, `read_at`, `write_at`     | Random access              |
| `FsLock`        | File locking          | `lock`, `unlock`, `try_lock`               | Multi-process coordination |
| `FsXattr`       | Extended attributes   | `get_xattr`, `set_xattr`, `list_xattr`     | Metadata storage           |
| `FsPath`        | Path canonicalization | `canonicalize`, `soft_canonicalize`        | Symlink resolution         |

### Composite Traits

| Trait     | Combines                                         | Use Case                        |
| --------- | ------------------------------------------------ | ------------------------------- |
| `Fs`      | `FsRead + FsWrite + FsDir`                       | 90% of apps                     |
| `FsFull`  | `Fs + FsLink + FsPermissions + FsSync + FsStats` | File managers, backup tools     |
| `FsFuse`  | `FsFull + FsInode`                               | FUSE implementations            |
| `FsPosix` | `FsFuse + FsHandles + FsLock + FsXattr`          | Databases, POSIX-compliant apps |

## Trait Object Usage

```rust
use anyfs_backend::Fs;
use std::path::Path;

fn process(fs: &dyn Fs) {
    let _ = fs.read(Path::new("/file.txt"));
}
```

## Error Handling

```rust
use anyfs_backend::{Fs, FsError};
use std::path::Path;

fn handle_errors<B: Fs>(fs: &B, path: &Path) -> Result<Vec<u8>, FsError> {
    match fs.read(path) {
        Ok(data) => Ok(data),
        Err(FsError::NotFound { path }) => {
            eprintln!("Not found: {}", path.display());
            Ok(Vec::new())
        }
        Err(FsError::PermissionDenied { path, operation }) => {
            eprintln!("Denied: {} on {}", operation, path.display());
            Err(FsError::PermissionDenied { path, operation })
        }
        Err(e) => Err(e),
    }
}
```

## FsError Variants

```rust
use anyfs_backend::FsError;
use std::path::PathBuf;

// Path errors
FsError::NotFound { path: PathBuf }
FsError::AlreadyExists { path: PathBuf, operation: &'static str }
FsError::NotAFile { path: PathBuf }
FsError::NotADirectory { path: PathBuf }
FsError::DirectoryNotEmpty { path: PathBuf }

// Permission errors  
FsError::PermissionDenied { path: PathBuf, operation: &'static str }
FsError::AccessDenied { path: PathBuf, reason: String }
FsError::ReadOnly { operation: &'static str }

// Resource errors
FsError::QuotaExceeded { limit: u64, requested: u64, usage: u64 }
FsError::FileSizeExceeded { path: PathBuf, size: u64, limit: u64 }

// Data errors
FsError::InvalidData { path: PathBuf, details: String }
FsError::CorruptedData { path: PathBuf, details: String }

// I/O error wrapper
FsError::Io { operation: &'static str, path: PathBuf, source: std::io::Error }
```

## Core Types

### Metadata

```rust
use anyfs_backend::{Metadata, FileType};

let meta = fs.metadata(path)?;

meta.file_type  // FileType::File | FileType::Directory | FileType::Symlink
meta.size       // u64 bytes
meta.permissions // Permissions
meta.modified   // Option<SystemTime>
meta.accessed   // Option<SystemTime>
meta.created    // Option<SystemTime>
meta.inode      // Option<u64>
meta.nlink      // Option<u64>
meta.uid        // Option<u32>
meta.gid        // Option<u32>
```

### FileType

```rust
use anyfs_backend::FileType;

FileType::File
FileType::Directory
FileType::Symlink
```

### DirEntry

```rust
use anyfs_backend::DirEntry;

for entry in fs.read_dir(path)? {
    let entry: DirEntry = entry?;
    entry.name       // String
    entry.path       // PathBuf
    entry.file_type  // Option<FileType>
    entry.inode      // Option<u64>
}
```

### Permissions

```rust
use anyfs_backend::Permissions;

Permissions::from_mode(0o755)
Permissions::readonly(true)
perms.mode()     // u32
perms.readonly() // bool
```

### StatFs

```rust
use anyfs_backend::StatFs;

let stats = fs.statfs()?;
stats.total_bytes     // u64
stats.free_bytes      // u64
stats.available_bytes // u64
stats.total_inodes    // Option<u64>
stats.free_inodes     // Option<u64>
```

## Implementing a Backend

```rust
use anyfs_backend::{Fs, FsRead, FsWrite, FsDir, FsError, Metadata, ReadDirIter};
use std::path::Path;
use std::io::{Read, Write};

struct MyBackend;

impl FsRead for MyBackend {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> { todo!() }
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> { todo!() }
    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> { todo!() }
    fn exists(&self, path: &Path) -> Result<bool, FsError> { todo!() }
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> { todo!() }
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> { todo!() }
}

impl FsWrite for MyBackend {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> { todo!() }
    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError> { todo!() }
    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> { todo!() }
    fn remove_file(&self, path: &Path) -> Result<(), FsError> { todo!() }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> { todo!() }
    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> { todo!() }
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError> { todo!() }
}

impl FsDir for MyBackend {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> { todo!() }
    fn create_dir(&self, path: &Path) -> Result<(), FsError> { todo!() }
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> { todo!() }
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> { todo!() }
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> { todo!() }
}

// MyBackend now implements Fs automatically via blanket impl
```

## Thread Safety

All traits require `Send + Sync`. Use interior mutability:

```rust
use std::sync::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;

struct ThreadSafeBackend {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
}

impl ThreadSafeBackend {
    fn read_impl(&self, path: &std::path::Path) -> Option<Vec<u8>> {
        self.files.read().unwrap().get(path).cloned()
    }
    
    fn write_impl(&self, path: &std::path::Path, data: Vec<u8>) {
        self.files.write().unwrap().insert(path.to_path_buf(), data);
    }
}
```

## Imports

```rust
// Basic usage
use anyfs_backend::{Fs, FsError};

// Extended traits
use anyfs_backend::{FsFull, FsFuse, FsPosix};

// Component traits
use anyfs_backend::{FsRead, FsWrite, FsDir};
use anyfs_backend::{FsLink, FsPermissions, FsSync, FsStats};
use anyfs_backend::{FsInode, FsHandles, FsLock, FsXattr};

// Types
use anyfs_backend::{Metadata, DirEntry, FileType, Permissions, StatFs};
use anyfs_backend::{Handle, OpenFlags, LockType, ROOT_INODE};
use anyfs_backend::ReadDirIter;
```

## Serde Feature

```toml
[dependencies]
anyfs-backend = { version = "0.1", features = ["serde"] }
```

```rust
use anyfs_backend::Metadata;

let json = serde_json::to_string(&metadata)?;
let meta: Metadata = serde_json::from_str(&json)?;
```

## Common Mistakes

### Parent directory must exist before write

```rust
// WRONG - fails with NotFound if /data doesn't exist
fs.write(Path::new("/data/file.txt"), b"content")?;

// CORRECT - create parents first
let path = Path::new("/data/file.txt");
if let Some(parent) = path.parent() {
    fs.create_dir_all(parent)?;
}
fs.write(path, b"content")?;
```

### FsError is non_exhaustive - always use wildcard

```rust
// WRONG - won't compile with future versions
match err {
    FsError::NotFound { .. } => {},
    FsError::PermissionDenied { .. } => {},
}

// CORRECT - always include catch-all
match err {
    FsError::NotFound { .. } => {},
    FsError::PermissionDenied { .. } => {},
    other => { /* handle unknown */ }
}
```

### Traits require &self, not &mut self

```rust
// WRONG
impl FsWrite for MyBackend {
    fn write(&mut self, path: &Path, data: &[u8]) -> Result<(), FsError> { ... }
}

// CORRECT - use interior mutability
struct MyBackend {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
}

impl FsWrite for MyBackend {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.files.write().unwrap().insert(path.into(), data.to_vec());
        Ok(())
    }
}
```

### Use correct error for each situation

```rust
// Reading non-existent file
Err(FsError::NotFound { path: path.into() })

// Reading a directory as a file
Err(FsError::NotAFile { path: path.into() })

// Listing a file as directory
Err(FsError::NotADirectory { path: path.into() })

// Creating dir that exists
Err(FsError::AlreadyExists { path: path.into(), operation: "create_dir" })

// Removing non-empty directory
Err(FsError::DirectoryNotEmpty { path: path.into() })

// No read/write permission
Err(FsError::PermissionDenied { path: path.into(), operation: "read" })
```

## Which Trait to Use

| Need                        | Trait     | Key Methods                                   |
| --------------------------- | --------- | --------------------------------------------- |
| Read/write files, list dirs | `Fs`      | `read`, `write`, `read_dir`, `create_dir_all` |
| + symlinks, permissions     | `FsFull`  | + `symlink`, `hard_link`, `set_permissions`   |
| + FUSE mounting             | `FsFuse`  | + `path_to_inode`, `metadata_by_inode`        |
| + file handles, locks       | `FsPosix` | + `open`, `close`, `lock`, `unlock`           |

Use the **smallest trait** that covers your needs. `Fs` covers 90% of use cases.

## ReadDirIter for Backend Implementers

```rust
use anyfs_backend::{ReadDirIter, DirEntry, FileType, FsError};
use std::path::PathBuf;

impl FsDir for MyBackend {
    fn read_dir(&self, path: &std::path::Path) -> Result<ReadDirIter, FsError> {
        let entries: Vec<Result<DirEntry, FsError>> = vec![
            Ok(DirEntry {
                name: "file.txt".into(),
                path: path.join("file.txt"),
                file_type: Some(FileType::File),
                inode: None,
            }),
            Ok(DirEntry {
                name: "subdir".into(),
                path: path.join("subdir"),
                file_type: Some(FileType::Directory),
                inode: None,
            }),
        ];
        Ok(ReadDirIter::from_vec(entries))
    }
}
```

## FsFuse - Inode Operations

```rust
use anyfs_backend::{FsFuse, FsInode, FsError, Metadata, ROOT_INODE};
use std::path::{Path, PathBuf};

fn fuse_ops<B: FsFuse>(fs: &B) -> Result<(), FsError> {
    // Convert path to inode
    let inode: u64 = fs.path_to_inode(Path::new("/file.txt"))?;
    
    // Convert inode back to path
    let path: PathBuf = fs.inode_to_path(inode)?;
    
    // Get metadata by inode (faster for FUSE)
    let meta: Metadata = fs.metadata_by_inode(inode)?;
    
    // Root is always inode 1
    assert_eq!(ROOT_INODE, 1);
    
    Ok(())
}
```

## FsPosix - Handles, Locks, Xattrs

```rust
use anyfs_backend::{FsPosix, FsHandles, FsLock, FsXattr, FsError, Handle, OpenFlags, LockType};
use std::path::Path;

fn posix_ops<B: FsPosix>(fs: &B) -> Result<(), FsError> {
    // Open file handle
    let handle: Handle = fs.open(Path::new("/file.txt"), OpenFlags::READ)?;
    
    // Read/write via handle (buffer-based API)
    let mut buf = [0u8; 1024];
    let bytes_read = fs.read_at(handle, &mut buf, 0)?;  // read at offset 0
    let bytes_written = fs.write_at(handle, b"data", 0)?;  // write at offset 0
    
    // File locking
    fs.lock(handle, LockType::Exclusive)?;
    fs.unlock(handle)?;
    
    // Close handle
    fs.close(handle)?;
    
    // Extended attributes
    fs.set_xattr(Path::new("/file.txt"), "user.key", b"value")?;
    let val = fs.get_xattr(Path::new("/file.txt"), "user.key")?;
    let names = fs.list_xattr(Path::new("/file.txt"))?;
    fs.remove_xattr(Path::new("/file.txt"), "user.key")?;
    
    Ok(())
}
```
