# FsWrite: Writing Files

`FsWrite` provides write operations for files.

## The Trait

```rust
pub trait FsWrite: Send + Sync {
    /// Write content to a file, creating it if it doesn't exist.
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;

    /// Append data to a file.
    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;

    /// Remove a file.
    fn remove_file(&self, path: &Path) -> Result<(), FsError>;

    /// Rename/move a file or directory.
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// Copy a file.
    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError>;

    /// Truncate a file to the specified size.
    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError>;

    /// Open a file for writing (returns boxed writer).
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError>;
}
```

## Implementation

### `write` - Write File Contents

```rust
use anyfs_backend::{FsWrite, FsError, FileType};

impl FsWrite for TutorialFs {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        let path = Self::normalize_path(path);
        let mut inner = self.inner.write().unwrap();

        // Check parent directory exists
        if let Some(parent) = path.parent() {
            let parent = Self::normalize_path(parent);
            match inner.nodes.get(&parent) {
                None => {
                    return Err(FsError::NotFound { path: parent });
                }
                Some(node) if node.file_type != FileType::Directory => {
                    return Err(FsError::NotADirectory { path: parent });
                }
                _ => {}
            }
        }

        // Can't write to a directory
        if let Some(existing) = inner.nodes.get(&path) {
            if existing.file_type == FileType::Directory {
                return Err(FsError::IsADirectory { path });
            }
        }

        // Create or update the file
        let inode = if let Some(existing) = inner.nodes.get(&path) {
            existing.inode  // Reuse existing inode
        } else {
            Self::alloc_inode(&mut inner)
        };

        let mut node = FsNode::new_file(content.to_vec(), inode);
        node.modified = SystemTime::now();

        inner.inode_to_path.insert(inode, path.clone());
        inner.nodes.insert(path, node);

        Ok(())
    }
    // ...
}
```

**Key points:**
- Verify parent directory exists before creating file
- Reuse inode if file already exists (overwrite)
- Update modification timestamp

### `remove_file` - Delete a File

```rust
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        let path = Self::normalize_path(path);
        let mut inner = self.inner.write().unwrap();

        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        // Can't remove directories with remove_file
        if node.file_type == FileType::Directory {
            return Err(FsError::IsADirectory { path });
        }

        let inode = node.inode;
        inner.nodes.remove(&path);
        inner.inode_to_path.remove(&inode);

        Ok(())
    }
```

## Error Handling

| Situation                            | Error                                     |
| ------------------------------------ | ----------------------------------------- |
| Parent directory doesn't exist       | `FsError::NotFound { path: parent }`      |
| Parent path is a file, not directory | `FsError::NotADirectory { path: parent }` |
| Target path is a directory           | `FsError::IsADirectory { path }`          |
| File to remove doesn't exist         | `FsError::NotFound { path }`              |

## Testing

```rust
#[test]
fn test_write_creates_file() {
    let fs = TutorialFs::new();
    
    fs.write(Path::new("/hello.txt"), b"Hello!").unwrap();
    
    let content = fs.read(Path::new("/hello.txt")).unwrap();
    assert_eq!(content, b"Hello!");
}

#[test]
fn test_write_overwrites_existing() {
    let fs = TutorialFs::new();
    
    fs.write(Path::new("/file.txt"), b"First").unwrap();
    fs.write(Path::new("/file.txt"), b"Second").unwrap();
    
    let content = fs.read(Path::new("/file.txt")).unwrap();
    assert_eq!(content, b"Second");
}

#[test]
fn test_write_to_nonexistent_parent_fails() {
    let fs = TutorialFs::new();
    
    let result = fs.write(Path::new("/no/such/dir/file.txt"), b"data");
    assert!(matches!(result, Err(FsError::NotFound { .. })));
}

#[test]
fn test_remove_file() {
    let fs = TutorialFs::new();
    
    fs.write(Path::new("/temp.txt"), b"temp").unwrap();
    assert!(fs.exists(Path::new("/temp.txt")));
    
    fs.remove_file(Path::new("/temp.txt")).unwrap();
    assert!(!fs.exists(Path::new("/temp.txt")));
}
```

## Summary

`FsWrite` provides:
- `write()` - Create or overwrite file contents
- `remove_file()` - Delete a file

Next, we'll implement [FsDir →](./04-fs-dir.md)
