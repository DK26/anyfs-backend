# FsInode: FUSE Support

`FsInode` provides inode-based operations, essential for FUSE (Filesystem in Userspace) implementations.

## Why Inodes?

FUSE operates on **inodes** rather than paths:

```
Path-based:  read("/home/user/file.txt")
Inode-based: read(inode=12345, offset=0, len=100)
```

Benefits:
- **Efficiency**: Inode lookup is O(1), path resolution is O(n)
- **Handles edge cases**: Deleted-but-open files, renamed files
- **FUSE requirement**: FUSE kernel module uses inodes

## The Trait

```rust
pub trait FsInode: Send + Sync {
    /// Convert a path to its inode number.
    fn path_to_inode(&self, path: &Path) -> Result<u64, FsError>;

    /// Convert an inode to its path.
    fn inode_to_path(&self, inode: u64) -> Result<PathBuf, FsError>;

    /// Get metadata by inode.
    fn metadata_by_inode(&self, inode: u64) -> Result<Metadata, FsError>;

    /// Look up child by name within a parent directory.
    fn lookup(&self, parent_inode: u64, name: &str) -> Result<u64, FsError>;
}
```

## The ROOT_INODE Constant

Root directory always has inode 1:

```rust
use anyfs_backend::ROOT_INODE;

assert_eq!(ROOT_INODE, 1);
```

## Implementation

### `path_to_inode`

```rust
impl FsInode for TutorialFs {
    fn path_to_inode(&self, path: &Path) -> Result<u64, FsError> {
        let path = Self::normalize_path(path);
        let inner = self.inner.read().unwrap();

        let node = inner.nodes.get(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        Ok(node.inode)
    }
    // ...
}
```

### `inode_to_path`

```rust
    fn inode_to_path(&self, inode: u64) -> Result<PathBuf, FsError> {
        let inner = self.inner.read().unwrap();

        inner.inode_to_path.get(&inode)
            .cloned()
            .ok_or(FsError::InodeNotFound { inode })
    }
```

Note the special error type `FsError::InodeNotFound`.

### `metadata_by_inode`

```rust
    fn metadata_by_inode(&self, inode: u64) -> Result<Metadata, FsError> {
        let inner = self.inner.read().unwrap();

        let path = inner.inode_to_path.get(&inode)
            .ok_or(FsError::InodeNotFound { inode })?;

        let node = inner.nodes.get(path)
            .ok_or(FsError::InodeNotFound { inode })?;

        Ok(node.to_metadata(path))
    }
```

### `lookup` - The Key FUSE Operation

This is how FUSE navigates directories:

```rust
    fn lookup(&self, parent_inode: u64, name: &str) -> Result<u64, FsError> {
        let inner = self.inner.read().unwrap();

        // Get parent path
        let parent_path = inner.inode_to_path.get(&parent_inode)
            .ok_or(FsError::InodeNotFound { inode: parent_inode })?;

        // Build child path
        let child_path = parent_path.join(name);

        // Look up child
        let child_node = inner.nodes.get(&child_path)
            .ok_or_else(|| FsError::NotFound { path: child_path })?;

        Ok(child_node.inode)
    }
```

## How FUSE Uses This

When a user accesses `/home/user/file.txt`, FUSE:

1. Starts at ROOT_INODE (1)
2. Calls `lookup(1, "home")` → inode 2
3. Calls `lookup(2, "user")` → inode 5
4. Calls `lookup(5, "file.txt")` → inode 12
5. Calls `metadata_by_inode(12)` → file metadata

## FsFuse Trait

`FsFuse` combines everything:

```rust
pub trait FsFuse: FsFull + FsInode {}
```

It's automatically implemented:

```rust
fn use_fs_fuse<B: FsFuse>(_: &B) {}

fn main() {
    let fs = TutorialFs::new();
    use_fs_fuse(&fs);  // ✅ Works!
}
```

## Testing

```rust
#[test]
fn test_path_to_inode() {
    let fs = TutorialFs::new();
    
    // Root is always inode 1
    let root_inode = fs.path_to_inode(Path::new("/")).unwrap();
    assert_eq!(root_inode, ROOT_INODE);
}

#[test]
fn test_inode_roundtrip() {
    let fs = TutorialFs::new();
    
    fs.create_dir(Path::new("/mydir")).unwrap();
    
    let inode = fs.path_to_inode(Path::new("/mydir")).unwrap();
    let path = fs.inode_to_path(inode).unwrap();
    
    assert_eq!(path, Path::new("/mydir"));
}

#[test]
fn test_lookup() {
    let fs = TutorialFs::new();
    
    fs.create_dir(Path::new("/parent")).unwrap();
    fs.write(Path::new("/parent/child.txt"), b"data").unwrap();
    
    let parent_inode = fs.path_to_inode(Path::new("/parent")).unwrap();
    let child_inode = fs.lookup(parent_inode, "child.txt").unwrap();
    
    let child_meta = fs.metadata_by_inode(child_inode).unwrap();
    assert_eq!(child_meta.file_type, FileType::File);
}

#[test]
fn test_lookup_from_root() {
    let fs = TutorialFs::new();
    
    fs.create_dir(Path::new("/documents")).unwrap();
    
    let docs_inode = fs.lookup(ROOT_INODE, "documents").unwrap();
    assert!(docs_inode > ROOT_INODE);  // Should be a new inode
}

#[test]
fn test_inode_not_found() {
    let fs = TutorialFs::new();
    
    let result = fs.inode_to_path(99999);
    assert!(matches!(result, Err(FsError::InodeNotFound { inode: 99999 })));
}
```

## Summary

`FsInode` provides inode-based access:
- `path_to_inode()` / `inode_to_path()` - Convert between paths and inodes
- `metadata_by_inode()` - Get metadata by inode
- `lookup()` - Find child in directory by name

`FsFuse = FsFull + FsInode` - Ready for FUSE implementation.

Next: [FsPosix: Full POSIX →](./09-fs-posix.md)
