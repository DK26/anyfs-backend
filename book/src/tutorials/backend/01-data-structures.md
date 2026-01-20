# Core Data Structures

Before implementing any traits, we need to design our internal data structures. These hold the actual filesystem state.

## Key Design Decisions

### 1. Thread Safety

All trait methods take `&self` (not `&mut self`), so we must use **interior mutability**:

```rust
// ❌ Won't work - traits don't allow &mut self
impl FsWrite for MyFs {
    fn write(&mut self, path: &Path, content: &[u8]) -> Result<(), FsError> { ... }
}

// ✅ Correct - use interior mutability
impl FsWrite for MyFs {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        let mut files = self.files.write().unwrap();  // RwLock
        files.insert(path.to_path_buf(), content.to_vec());
        Ok(())
    }
}
```

We wrap mutable state in `RwLock` (or `Mutex`).

### 2. Path Normalization

Paths like `/foo/bar`, `/foo//bar`, and `/foo/./bar` should all refer to the same file. Always normalize before using as keys:

```rust
fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => result = PathBuf::from("/"),
            Component::CurDir => {}  // Skip "."
            Component::ParentDir => { result.pop(); }  // Handle ".."
            Component::Normal(name) => { result.push(name); }
            Component::Prefix(_) => {}  // Windows, skip
        }
    }
    result
}
```

### 3. Inode Design

Even if you only need `Fs`, designing with inodes from the start makes it easier to add `FsFuse` later:

```rust
struct FsNode {
    inode: u64,  // Unique identifier
    // ... other fields
}
```

## The FsNode Structure

Each node in our filesystem (file, directory, or symlink) has:

```rust
use anyfs_backend::{FileType, Permissions};
use std::path::PathBuf;
use std::time::SystemTime;

/// Represents a single node in the filesystem.
#[derive(Clone)]
struct FsNode {
    /// Type: File, Directory, or Symlink
    file_type: FileType,

    /// File contents (empty for directories)
    content: Vec<u8>,

    /// Permission bits (e.g., 0o644)
    permissions: Permissions,

    /// Symlink target (only for symlinks)
    symlink_target: Option<PathBuf>,

    /// Unique inode number
    inode: u64,

    /// Timestamps
    created: SystemTime,
    modified: SystemTime,
    accessed: SystemTime,
}
```

### Factory Methods

Create nodes easily:

```rust
impl FsNode {
    fn new_file(content: Vec<u8>, inode: u64) -> Self {
        let now = SystemTime::now();
        Self {
            file_type: FileType::File,
            content,
            permissions: Permissions::from_mode(0o644),  // rw-r--r--
            symlink_target: None,
            inode,
            created: now,
            modified: now,
            accessed: now,
        }
    }

    fn new_directory(inode: u64) -> Self {
        let now = SystemTime::now();
        Self {
            file_type: FileType::Directory,
            content: Vec::new(),
            permissions: Permissions::from_mode(0o755),  // rwxr-xr-x
            symlink_target: None,
            inode,
            created: now,
            modified: now,
            accessed: now,
        }
    }

    fn new_symlink(target: PathBuf, inode: u64) -> Self {
        let now = SystemTime::now();
        Self {
            file_type: FileType::Symlink,
            content: Vec::new(),
            permissions: Permissions::from_mode(0o777),  // lrwxrwxrwx
            symlink_target: Some(target),
            inode,
            created: now,
            modified: now,
            accessed: now,
        }
    }
}
```

## The Inner State

All mutable state goes in a struct wrapped by `RwLock`:

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use anyfs_backend::{Handle, OpenFlags, LockType};

struct TutorialFsInner {
    /// Maps normalized paths to nodes
    nodes: HashMap<PathBuf, FsNode>,

    /// Maps inodes to paths (for inode-based lookups)
    inode_to_path: HashMap<u64, PathBuf>,

    /// Next available inode number
    next_inode: u64,

    /// Open file handles (for FsHandles)
    handles: HashMap<Handle, HandleState>,

    /// Next available handle ID
    next_handle: u64,

    /// Total filesystem size for stats
    total_size: u64,
}

/// State for an open file handle.
struct HandleState {
    path: PathBuf,
    flags: OpenFlags,
    locked: Option<LockType>,
}
```

## The Public Backend Type

The main struct wraps everything in `Arc<RwLock<_>>`:

```rust
use std::sync::{Arc, RwLock};

/// Our tutorial filesystem backend.
pub struct TutorialFs {
    inner: Arc<RwLock<TutorialFsInner>>,
}

impl TutorialFs {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        let mut inode_to_path = HashMap::new();

        // Always create root directory with inode 1 (ROOT_INODE)
        use anyfs_backend::ROOT_INODE;
        let root = FsNode::new_directory(ROOT_INODE);
        nodes.insert(PathBuf::from("/"), root);
        inode_to_path.insert(ROOT_INODE, PathBuf::from("/"));

        Self {
            inner: Arc::new(RwLock::new(TutorialFsInner {
                nodes,
                inode_to_path,
                next_inode: 2,  // Start after ROOT_INODE
                next_handle: 1,
                handles: HashMap::new(),
                total_size: 100 * 1024 * 1024,  // 100 MB virtual size
            })),
        }
    }
}
```

## Helper Methods

Add utility methods for common operations:

```rust
impl TutorialFs {
    /// Normalize a path for consistent storage and lookup.
    fn normalize_path(path: &Path) -> PathBuf {
        // Implementation from above
    }

    /// Allocate a new inode number.
    fn alloc_inode(inner: &mut TutorialFsInner) -> u64 {
        let inode = inner.next_inode;
        inner.next_inode += 1;
        inode
    }

    /// Allocate a new file handle.
    fn alloc_handle(inner: &mut TutorialFsInner) -> Handle {
        let id = inner.next_handle;
        inner.next_handle += 1;
        Handle(id)
    }
}
```

## Converting to Metadata

The traits return `Metadata` structs. Add a conversion method:

```rust
use anyfs_backend::Metadata;

impl FsNode {
    fn to_metadata(&self, path: &Path) -> Metadata {
        Metadata {
            path: path.to_path_buf(),
            file_type: self.file_type,
            len: self.content.len() as u64,
            permissions: self.permissions.clone(),
            created: Some(self.created),
            modified: Some(self.modified),
            accessed: Some(self.accessed),
            inode: Some(self.inode),
            uid: Some(1000),
            gid: Some(1000),
            nlink: Some(1),
        }
    }
}
```

## Summary

We now have:

| Type              | Purpose                                  |
| ----------------- | ---------------------------------------- |
| `FsNode`          | Represents a file, directory, or symlink |
| `TutorialFsInner` | All mutable state                        |
| `TutorialFs`      | Public backend with `Arc<RwLock<Inner>>` |

Next, we'll implement [FsRead →](./02-fs-read.md)
