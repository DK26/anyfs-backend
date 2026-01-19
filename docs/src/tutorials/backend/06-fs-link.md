# FsLink: Symlinks

`FsLink` adds symbolic and hard link support to your backend.

## The Trait

```rust
pub trait FsLink: Send + Sync {
    /// Create a symbolic link.
    fn symlink(&self, target: &Path, link: &Path) -> Result<(), FsError>;

    /// Read the target of a symbolic link.
    fn read_link(&self, path: &Path) -> Result<PathBuf, FsError>;

    /// Create a hard link.
    fn hard_link(&self, target: &Path, link: &Path) -> Result<(), FsError>;
}
```

## Understanding Symlinks vs Hard Links

| Aspect            | Symlink                     | Hard Link                |
| ----------------- | --------------------------- | ------------------------ |
| What it stores    | Path to target              | Same inode as target     |
| Target can be...  | Anything (even nonexistent) | Must exist and be a file |
| Cross-filesystem  | Yes                         | No                       |
| If target deleted | Becomes broken              | File still accessible    |

## Implementation

### `symlink` - Create Symbolic Link

```rust
impl FsLink for TutorialFs {
    fn symlink(&self, target: &Path, link: &Path) -> Result<(), FsError> {
        let link = Self::normalize_path(link);
        let mut inner = self.inner.write().unwrap();

        // Check if link path already exists
        if inner.nodes.contains_key(&link) {
            return Err(FsError::AlreadyExists { path: link });
        }

        // Check parent directory exists
        if let Some(parent) = link.parent() {
            let parent = Self::normalize_path(parent);
            if !inner.nodes.contains_key(&parent) {
                return Err(FsError::NotFound { path: parent });
            }
        }

        // Create the symlink node
        // Note: target is stored as-is, not normalized
        let inode = Self::alloc_inode(&mut inner);
        let node = FsNode::new_symlink(target.to_path_buf(), inode);
        inner.inode_to_path.insert(inode, link.clone());
        inner.nodes.insert(link, node);

        Ok(())
    }
    // ...
}
```

**Key points:**
- The `target` path is stored as-is (can be relative or absolute)
- The target doesn't need to exist
- The `link` path must not already exist

### `read_link` - Get Symlink Target

```rust
    fn read_link(&self, path: &Path) -> Result<PathBuf, FsError> {
        let path = Self::normalize_path(path);
        let inner = self.inner.read().unwrap();

        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        match &node.symlink_target {
            Some(target) => Ok(target.clone()),
            None => Err(FsError::InvalidData {
                details: format!("{} is not a symbolic link", path.display()),
            }),
        }
    }
```

### `hard_link` - Create Hard Link

Hard links are more complex because they share inodes. For simplicity, you can return `Unsupported`:

```rust
    fn hard_link(&self, _target: &Path, _link: &Path) -> Result<(), FsError> {
        Err(FsError::Unsupported {
            operation: "hard_link".to_string(),
        })
    }
```

Or implement properly by having multiple paths point to the same inode:

```rust
    fn hard_link(&self, target: &Path, link: &Path) -> Result<(), FsError> {
        let target = Self::normalize_path(target);
        let link = Self::normalize_path(link);
        let mut inner = self.inner.write().unwrap();

        // Target must exist and be a file
        let target_node = inner.nodes.get(&target)
            .ok_or_else(|| FsError::NotFound { path: target.clone() })?;
        
        if target_node.file_type != FileType::File {
            return Err(FsError::InvalidData {
                details: "hard links can only target files".to_string(),
            });
        }

        // Link must not exist
        if inner.nodes.contains_key(&link) {
            return Err(FsError::AlreadyExists { path: link });
        }

        // Clone the node (same content, same inode)
        let mut link_node = target_node.clone();
        // Note: In a real impl, you'd track nlink count
        
        inner.nodes.insert(link.clone(), link_node);
        // Don't add to inode_to_path - inode already maps to target

        Ok(())
    }
```

## Symlink Resolution

When reading through a symlink, you may need to resolve it:

```rust
/// Resolve a path, following symlinks.
fn resolve_path<B: Fs + FsLink>(fs: &B, path: &Path) -> Result<PathBuf, FsError> {
    let mut current = path.to_path_buf();
    let mut seen = std::collections::HashSet::new();

    loop {
        // Prevent infinite loops
        if !seen.insert(current.clone()) {
            return Err(FsError::InvalidData {
                details: "symlink loop detected".to_string(),
            });
        }

        match fs.metadata(&current) {
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
```

## Testing

```rust
#[test]
fn test_symlink_creation() {
    let fs = TutorialFs::new();
    
    fs.write(Path::new("/original.txt"), b"content").unwrap();
    fs.symlink(Path::new("/original.txt"), Path::new("/link.txt")).unwrap();
    
    // Check metadata shows it's a symlink
    let meta = fs.metadata(Path::new("/link.txt")).unwrap();
    assert_eq!(meta.file_type, FileType::Symlink);
    
    // Read the link target
    let target = fs.read_link(Path::new("/link.txt")).unwrap();
    assert_eq!(target, Path::new("/original.txt"));
}

#[test]
fn test_symlink_to_nonexistent() {
    let fs = TutorialFs::new();
    
    // This should succeed - symlinks can point to nonexistent targets
    fs.symlink(Path::new("/nonexistent"), Path::new("/broken-link")).unwrap();
    
    let target = fs.read_link(Path::new("/broken-link")).unwrap();
    assert_eq!(target, Path::new("/nonexistent"));
}

#[test]
fn test_read_link_on_regular_file_fails() {
    let fs = TutorialFs::new();
    
    fs.write(Path::new("/file.txt"), b"data").unwrap();
    
    let result = fs.read_link(Path::new("/file.txt"));
    assert!(matches!(result, Err(FsError::InvalidData { .. })));
}
```

## Summary

`FsLink` provides:
- `symlink()` - Create symbolic links
- `read_link()` - Read symlink target
- `hard_link()` - Create hard links (optional)

Next: [FsFull: Complete Filesystem →](./07-fs-full.md)
