# FsPosix: Full POSIX Semantics

`FsPosix` is the final layer, adding file handles, locking, and extended attributes for complete POSIX semantics.

## The Trait

```rust
pub trait FsPosix: FsFuse + FsHandles + FsLock + FsXattr {}
```

## Component Traits

### FsHandles - File Handle Operations

Instead of reading/writing entire files, handles allow:
- Opening a file once, performing many operations
- Reading/writing at specific offsets
- Keeping files open across operations

```rust
pub trait FsHandles: Send + Sync {
    /// Open a file and return a handle.
    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Handle, FsError>;

    /// Read from a file at a specific offset into a buffer.
    fn read_at(&self, handle: Handle, buf: &mut [u8], offset: u64) -> Result<usize, FsError>;

    /// Write data to a file at a specific offset.
    fn write_at(&self, handle: Handle, data: &[u8], offset: u64) -> Result<usize, FsError>;

    /// Close a file handle.
    fn close(&self, handle: Handle) -> Result<(), FsError>;
}
```

#### OpenFlags

```rust
/// File open flags (struct, not bitflags).
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
}

impl OpenFlags {
    pub const READ: Self = Self { read: true, write: false, create: false, truncate: false, append: false };
    pub const WRITE: Self = Self { read: false, write: true, create: true, truncate: true, append: false };
    pub const READ_WRITE: Self = Self { read: true, write: true, create: false, truncate: false, append: false };
    pub const APPEND: Self = Self { read: false, write: true, create: true, truncate: false, append: true };
}
```

#### Implementation

```rust
impl FsHandles for TutorialFs {
    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Handle, FsError> {
        let path = Self::normalize_path(path);
        let mut inner = self.inner.write().unwrap();

        let exists = inner.nodes.contains_key(&path);

        // Handle creation
        if flags.create && !exists {
            let inode = Self::alloc_inode(&mut inner);
            let node = FsNode::new_file(Vec::new(), inode);
            inner.inode_to_path.insert(inode, path.clone());
            inner.nodes.insert(path.clone(), node);
        } else if !exists {
            return Err(FsError::NotFound { path });
        }

        // Truncate if requested
        if flags.truncate {
            if let Some(node) = inner.nodes.get_mut(&path) {
                node.content.clear();
            }
        }

        // Allocate handle
        let handle = Self::alloc_handle(&mut inner);
        inner.handles.insert(handle, HandleState {
            path,
            flags,
            locked: None,
        });

        Ok(handle)
    }

    fn read_at(&self, handle: Handle, buf: &mut [u8], offset: u64) -> Result<usize, FsError> {
        let inner = self.inner.read().unwrap();

        let state = inner.handles.get(&handle)
            .ok_or(FsError::InvalidHandle { handle })?;

        // Check read permission
        if !state.flags.read {
            return Err(FsError::PermissionDenied { 
                path: state.path.clone(),
                operation: "read_at"
            });
        }

        let node = inner.nodes.get(&state.path)
            .ok_or_else(|| FsError::NotFound { path: state.path.clone() })?;

        let start = offset as usize;
        if start >= node.content.len() {
            return Ok(0);  // EOF
        }

        let available = node.content.len() - start;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&node.content[start..start + to_read]);
        Ok(to_read)
    }

    fn write_at(&self, handle: Handle, data: &[u8], offset: u64) -> Result<usize, FsError> {
        let mut inner = self.inner.write().unwrap();

        // Get path from handle
        let path = {
            let state = inner.handles.get(&handle)
                .ok_or(FsError::InvalidHandle { handle })?;

            if !state.flags.contains(OpenFlags::WRITE) {
                return Err(FsError::PermissionDenied { path: state.path.clone() });
            }
            state.path.clone()
        };

        let node = inner.nodes.get_mut(&path)
            .ok_or_else(|| FsError::NotFound { path })?;

        let start = offset as usize;

        // Extend file if necessary
        if start + data.len() > node.content.len() {
            node.content.resize(start + data.len(), 0);
        }

        node.content[start..start + data.len()].copy_from_slice(data);
        node.modified = SystemTime::now();

        Ok(data.len())
    }
}
```

### FsLock - File Locking

Prevents concurrent access conflicts:

```rust
pub trait FsLock: Send + Sync {
    /// Acquire a lock (blocks until available).
    fn lock(&self, handle: Handle, lock_type: LockType) -> Result<(), FsError>;

    /// Try to acquire a lock (non-blocking).
    fn try_lock(&self, handle: Handle, lock_type: LockType) -> Result<bool, FsError>;

    /// Release a lock.
    fn unlock(&self, handle: Handle) -> Result<(), FsError>;
}

pub enum LockType {
    Shared,     // Multiple readers allowed
    Exclusive,  // Single writer, no readers
}
```

#### Implementation

```rust
impl FsLock for TutorialFs {
    fn lock(&self, handle: Handle, lock_type: LockType) -> Result<(), FsError> {
        let mut inner = self.inner.write().unwrap();

        let state = inner.handles.get_mut(&handle)
            .ok_or(FsError::InvalidHandle { handle })?;

        // Simple implementation: just record the lock
        // Real implementation would check for conflicts
        state.locked = Some(lock_type);

        Ok(())
    }

    fn try_lock(&self, handle: Handle, lock_type: LockType) -> Result<bool, FsError> {
        // For simplicity, always succeed
        self.lock(handle, lock_type)?;
        Ok(true)
    }

    fn unlock(&self, handle: Handle) -> Result<(), FsError> {
        let mut inner = self.inner.write().unwrap();

        let state = inner.handles.get_mut(&handle)
            .ok_or(FsError::InvalidHandle { handle })?;

        state.locked = None;

        Ok(())
    }
}
```

### FsXattr - Extended Attributes

Store arbitrary metadata on files (like Linux xattr):

```rust
pub trait FsXattr: Send + Sync {
    fn get_xattr(&self, path: &Path, name: &str) -> Result<Vec<u8>, FsError>;
    fn set_xattr(&self, path: &Path, name: &str, value: &[u8]) -> Result<(), FsError>;
    fn remove_xattr(&self, path: &Path, name: &str) -> Result<(), FsError>;
    fn list_xattr(&self, path: &Path) -> Result<Vec<String>, FsError>;
}
```

For simplicity, you can return `Unsupported`:

```rust
impl FsXattr for TutorialFs {
    fn get_xattr(&self, _path: &Path, _name: &str) -> Result<Vec<u8>, FsError> {
        Err(FsError::Unsupported { operation: "xattr".to_string() })
    }
    // ... same for other methods
}
```

## Putting It Together

With all traits implemented, verify `FsPosix`:

```rust
fn use_fs_posix<B: FsPosix>(_: &B) {}

fn main() {
    let fs = TutorialFs::new();
    use_fs_posix(&fs);  // ✅ Full POSIX support!
}
```

## Usage Example

```rust
use anyfs_backend::{FsPosix, OpenFlags, LockType, FsError};

fn atomic_update<B: FsPosix>(
    fs: &B,
    path: &Path,
    updater: impl FnOnce(&[u8]) -> Vec<u8>,
) -> Result<(), FsError> {
    // Open with read/write
    let handle = fs.open(path, OpenFlags::READ | OpenFlags::WRITE)?;
    
    // Lock exclusively
    fs.lock(handle, LockType::Exclusive)?;
    
    // Read current content
    let current = fs.read_at(handle, 0, usize::MAX)?;
    
    // Apply update
    let new_content = updater(&current);
    
    // Write back (truncate by writing from offset 0)
    fs.write_at(handle, 0, &new_content)?;
    
    // Unlock and close
    fs.unlock(handle)?;
    fs.close(handle)?;
    
    Ok(())
}
```

## Testing

```rust
#[test]
fn test_handle_read_write() {
    let fs = TutorialFs::new();
    
    // Create and open file
    let handle = fs.open(
        Path::new("/file.txt"),
        OpenFlags::READ | OpenFlags::WRITE | OpenFlags::CREATE,
    ).unwrap();
    
    // Write
    fs.write_at(handle, 0, b"Hello, World!").unwrap();
    
    // Read back
    let data = fs.read_at(handle, 0, 5).unwrap();
    assert_eq!(data, b"Hello");
    
    // Read with offset
    let data = fs.read_at(handle, 7, 5).unwrap();
    assert_eq!(data, b"World");
    
    fs.close(handle).unwrap();
}

#[test]
fn test_locking() {
    let fs = TutorialFs::new();
    
    let handle = fs.open(
        Path::new("/locked.txt"),
        OpenFlags::WRITE | OpenFlags::CREATE,
    ).unwrap();
    
    fs.lock(handle, LockType::Exclusive).unwrap();
    fs.write_at(handle, 0, b"Protected data").unwrap();
    fs.unlock(handle).unwrap();
    
    fs.close(handle).unwrap();
}

#[test]
fn test_invalid_handle() {
    let fs = TutorialFs::new();
    
    let invalid = Handle(99999);
    
    assert!(matches!(
        fs.read_at(invalid, 0, 10),
        Err(FsError::InvalidHandle { .. })
    ));
}
```

## Summary

You've implemented a complete filesystem backend!

| Layer       | Traits                                         |
| ----------- | ---------------------------------------------- |
| **Fs**      | FsRead + FsWrite + FsDir                       |
| **FsFull**  | Fs + FsLink + FsPermissions + FsSync + FsStats |
| **FsFuse**  | FsFull + FsInode                               |
| **FsPosix** | FsFuse + FsHandles + FsLock + FsXattr          |

Your `TutorialFs` now supports:
- ✅ File read/write
- ✅ Directory operations
- ✅ Symlinks
- ✅ Permissions
- ✅ Filesystem stats
- ✅ Inode-based access
- ✅ File handles
- ✅ File locking

🎉 **Congratulations!** You've built a full-featured filesystem backend.

Next, learn how to create middleware: [Implementing Middleware →](../middleware/README.md)
