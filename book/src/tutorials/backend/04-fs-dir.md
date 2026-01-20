# FsDir: Directory Operations

`FsDir` provides directory operations: listing, creating, and removing.

## The Trait

```rust
pub trait FsDir: Send + Sync {
    /// List directory contents.
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError>;

    /// Create a single directory.
    fn create_dir(&self, path: &Path) -> Result<(), FsError>;

    /// Create directory and all parent directories.
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError>;

    /// Remove an empty directory.
    fn remove_dir(&self, path: &Path) -> Result<(), FsError>;

    /// Remove directory and all contents recursively.
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError>;
}
```

> **Note:** `rename()` is in `FsWrite`, not `FsDir`.

## The ReadDirIter Type

`read_dir` returns a `ReadDirIter`, which is a boxed iterator over `Result<DirEntry, FsError>`:

```rust
// DirEntry contains info about each directory entry
pub struct DirEntry {
    pub path: PathBuf,       // Full path
    pub name: String,        // Just the filename
    pub file_type: FileType, // File, Directory, or Symlink
    pub inode: Option<u64>,  // Inode if available
}

// ReadDirIter is an iterator
pub struct ReadDirIter(Box<dyn Iterator<Item = Result<DirEntry, FsError>> + Send>);
```

Create a `ReadDirIter` from a vector:

```rust
let entries = vec![
    Ok(DirEntry { path: PathBuf::from("/foo"), name: "foo".into(), ... }),
    Ok(DirEntry { path: PathBuf::from("/bar"), name: "bar".into(), ... }),
];
ReadDirIter::from_vec(entries)
```

## Implementation

### `read_dir` - List Directory Contents

```rust
impl FsDir for TutorialFs {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        let path = Self::normalize_path(path);
        let inner = self.inner.read().unwrap();

        // Verify path exists and is a directory
        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        if node.file_type != FileType::Directory {
            return Err(FsError::NotADirectory { path });
        }

        // Collect direct children
        let mut entries = Vec::new();
        for (child_path, child_node) in &inner.nodes {
            if let Some(parent) = child_path.parent() {
                if Self::normalize_path(parent) == path && child_path != &path {
                    let name = child_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    entries.push(Ok(DirEntry {
                        path: child_path.clone(),
                        name,
                        file_type: child_node.file_type,
                        inode: Some(child_node.inode),
                    }));
                }
            }
        }

        // Sort for consistent ordering
        entries.sort_by_key(|e| e.as_ref().map(|e| e.name.clone()).ok());

        Ok(ReadDirIter::from_vec(entries))
    }
    // ...
}
```

### `create_dir` - Create Single Directory

```rust
    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = Self::normalize_path(path);
        let mut inner = self.inner.write().unwrap();

        // Check if already exists
        if inner.nodes.contains_key(&path) {
            return Err(FsError::AlreadyExists { path });
        }

        // Check parent exists and is a directory
        if let Some(parent) = path.parent() {
            let parent = Self::normalize_path(parent);
            match inner.nodes.get(&parent) {
                None => return Err(FsError::NotFound { path: parent }),
                Some(node) if node.file_type != FileType::Directory => {
                    return Err(FsError::NotADirectory { path: parent });
                }
                _ => {}
            }
        }

        let inode = Self::alloc_inode(&mut inner);
        let node = FsNode::new_directory(inode);
        inner.inode_to_path.insert(inode, path.clone());
        inner.nodes.insert(path, node);

        Ok(())
    }
```

### `create_dir_all` - Create Directory Tree

```rust
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        let path = Self::normalize_path(path);

        // Build list of directories to create (from root to leaf)
        let mut to_create = Vec::new();
        let mut current = path.clone();

        while current != Path::new("/") {
            if !self.exists(&current) {
                to_create.push(current.clone());
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }

        // Create from root towards leaf
        to_create.reverse();
        for dir in to_create {
            match self.create_dir(&dir) {
                Ok(()) | Err(FsError::AlreadyExists { .. }) => {}
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
```

**Note:** `create_dir_all` is idempotent—it succeeds even if the directory exists.

### `remove_dir` - Remove Empty Directory

```rust
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        let path = Self::normalize_path(path);
        let mut inner = self.inner.write().unwrap();

        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        if node.file_type != FileType::Directory {
            return Err(FsError::NotADirectory { path });
        }

        // Check if directory is empty
        for other_path in inner.nodes.keys() {
            if let Some(parent) = other_path.parent() {
                if Self::normalize_path(parent) == path {
                    return Err(FsError::DirectoryNotEmpty { path });
                }
            }
        }

        let inode = node.inode;
        inner.nodes.remove(&path);
        inner.inode_to_path.remove(&inode);

        Ok(())
    }
```

### `remove_dir_all` - Remove Recursively

```rust
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        let path = Self::normalize_path(path);
        let mut inner = self.inner.write().unwrap();

        // Verify it exists and is a directory
        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        if node.file_type != FileType::Directory {
            return Err(FsError::NotADirectory { path });
        }

        // Collect all paths to remove
        let to_remove: Vec<PathBuf> = inner.nodes.keys()
            .filter(|p| p.starts_with(&path))
            .cloned()
            .collect();

        for p in to_remove {
            if let Some(node) = inner.nodes.remove(&p) {
                inner.inode_to_path.remove(&node.inode);
            }
        }

        Ok(())
    }
```

### `rename` - Move/Rename

```rust
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let from = Self::normalize_path(from);
        let to = Self::normalize_path(to);
        let mut inner = self.inner.write().unwrap();

        if !inner.nodes.contains_key(&from) {
            return Err(FsError::NotFound { path: from });
        }

        if inner.nodes.contains_key(&to) {
            return Err(FsError::AlreadyExists { path: to });
        }

        // Check destination parent exists
        if let Some(parent) = to.parent() {
            let parent = Self::normalize_path(parent);
            if !inner.nodes.contains_key(&parent) {
                return Err(FsError::NotFound { path: parent });
            }
        }

        if let Some(node) = inner.nodes.remove(&from) {
            inner.inode_to_path.insert(node.inode, to.clone());
            inner.nodes.insert(to, node);
        }

        Ok(())
    }
```

## Testing

```rust
#[test]
fn test_create_and_list_dir() {
    let fs = TutorialFs::new();
    
    fs.create_dir(Path::new("/subdir")).unwrap();
    fs.write(Path::new("/subdir/file.txt"), b"content").unwrap();
    
    let entries: Vec<_> = fs.read_dir(Path::new("/subdir"))
        .unwrap()
        .collect();
    
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].as_ref().unwrap().name, "file.txt");
}

#[test]
fn test_create_dir_all() {
    let fs = TutorialFs::new();
    
    fs.create_dir_all(Path::new("/a/b/c/d")).unwrap();
    
    assert!(fs.exists(Path::new("/a")));
    assert!(fs.exists(Path::new("/a/b")));
    assert!(fs.exists(Path::new("/a/b/c")));
    assert!(fs.exists(Path::new("/a/b/c/d")));
}

#[test]
fn test_remove_nonempty_dir_fails() {
    let fs = TutorialFs::new();
    
    fs.create_dir(Path::new("/dir")).unwrap();
    fs.write(Path::new("/dir/file.txt"), b"data").unwrap();
    
    let result = fs.remove_dir(Path::new("/dir"));
    assert!(matches!(result, Err(FsError::DirectoryNotEmpty { .. })));
}
```

## Summary

`FsDir` provides:
- `read_dir()` - List directory entries
- `create_dir()` / `create_dir_all()` - Create directories
- `remove_dir()` / `remove_dir_all()` - Remove directories
- `rename()` - Move or rename entries

Next: [The Fs Trait →](./05-fs-trait.md)
