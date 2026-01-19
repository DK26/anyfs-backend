# FsRead: Reading Files

`FsRead` provides read-only access to files and metadata. This is the foundation for all filesystem access.

## The Trait

```rust
pub trait FsRead: Send + Sync {
    /// Read the entire contents of a file.
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;

    /// Get metadata (size, type, timestamps, etc.).
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError>;

    /// Check if a path exists.
    fn exists(&self, path: &Path) -> bool;
}
```

## Implementation

### `read` - Read File Contents

```rust
use anyfs_backend::{FsRead, FsError, FileType};

impl FsRead for TutorialFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let path = Self::normalize_path(path);
        let inner = self.inner.read().unwrap();

        // Look up the node
        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        // Directories can't be read as files
        if node.file_type == FileType::Directory {
            return Err(FsError::IsADirectory { path });
        }

        Ok(node.content.clone())
    }
    // ...
}
```

**Key points:**
- Normalize the path first for consistent lookup
- Return `FsError::NotFound` if the path doesn't exist
- Return `FsError::IsADirectory` if trying to read a directory

### `metadata` - Get File Information

```rust
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let path = Self::normalize_path(path);
        let inner = self.inner.read().unwrap();

        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        Ok(node.to_metadata(&path))
    }
```

The `Metadata` struct contains:

| Field         | Type                 | Description                 |
| ------------- | -------------------- | --------------------------- |
| `path`        | `PathBuf`            | The queried path            |
| `file_type`   | `FileType`           | File, Directory, or Symlink |
| `len`         | `u64`                | Size in bytes               |
| `permissions` | `Permissions`        | Permission bits             |
| `created`     | `Option<SystemTime>` | Creation time               |
| `modified`    | `Option<SystemTime>` | Last modification           |
| `accessed`    | `Option<SystemTime>` | Last access                 |
| `inode`       | `Option<u64>`        | Inode number                |
| `uid`         | `Option<u32>`        | Owner user ID               |
| `gid`         | `Option<u32>`        | Owner group ID              |
| `nlink`       | `Option<u32>`        | Hard link count             |

### `exists` - Check Path Existence

```rust
    fn exists(&self, path: &Path) -> bool {
        let path = Self::normalize_path(path);
        let inner = self.inner.read().unwrap();
        inner.nodes.contains_key(&path)
    }
```

**Important:** `exists` never fails. It returns `false` for any error condition.

## Error Handling Guidelines

| Situation                              | Error to return                      |
| -------------------------------------- | ------------------------------------ |
| Path doesn't exist                     | `FsError::NotFound { path }`         |
| Path is a directory when file expected | `FsError::IsADirectory { path }`     |
| Permission denied                      | `FsError::PermissionDenied { path }` |

Always include the path in error context so callers know what failed.

## Testing Your Implementation

```rust
#[test]
fn test_read_existing_file() {
    let fs = TutorialFs::new();
    
    // Setup: Create a file (we'll implement write later)
    {
        let mut inner = fs.inner.write().unwrap();
        let inode = TutorialFs::alloc_inode(&mut inner);
        let node = FsNode::new_file(b"Hello, World!".to_vec(), inode);
        inner.nodes.insert(PathBuf::from("/test.txt"), node);
    }
    
    // Test read
    let content = fs.read(Path::new("/test.txt")).unwrap();
    assert_eq!(content, b"Hello, World!");
}

#[test]
fn test_read_nonexistent_file() {
    let fs = TutorialFs::new();
    
    let result = fs.read(Path::new("/nonexistent.txt"));
    assert!(matches!(result, Err(FsError::NotFound { .. })));
}

#[test]
fn test_read_directory_fails() {
    let fs = TutorialFs::new();
    
    // Root directory exists
    let result = fs.read(Path::new("/"));
    assert!(matches!(result, Err(FsError::IsADirectory { .. })));
}

#[test]
fn test_exists() {
    let fs = TutorialFs::new();
    
    assert!(fs.exists(Path::new("/")));  // Root always exists
    assert!(!fs.exists(Path::new("/nonexistent")));
}
```

## Summary

`FsRead` provides:
- `read()` - Get file contents
- `metadata()` - Get file/directory information
- `exists()` - Quick existence check

Next, we'll implement [FsWrite →](./03-fs-write.md)
