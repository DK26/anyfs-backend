# FsFull: Complete Filesystem

`FsFull` adds permissions, sync, and stats to reach "complete" filesystem semantics.

## The Trait

```rust
// FsFull = Fs + FsLink + FsPermissions + FsSync + FsStats
pub trait FsFull: Fs + FsLink + FsPermissions + FsSync + FsStats {}
```

Like `Fs`, it's **automatically implemented** via blanket impl.

## Component Traits

### FsPermissions

```rust
pub trait FsPermissions: Send + Sync {
    /// Set file/directory permissions.
    fn set_permissions(&self, path: &Path, perm: Permissions) -> Result<(), FsError>;
}
```

> **Note:** Reading permissions is done via `FsRead::metadata()`. This trait only provides the ability to set permissions.

Implementation:

```rust
impl FsPermissions for TutorialFs {
    fn set_permissions(&self, path: &Path, perm: Permissions) -> Result<(), FsError> {
        let path = Self::normalize_path(path);
        let mut inner = self.inner.write().unwrap();

        let node = inner.nodes.get_mut(&path)
            .ok_or_else(|| FsError::NotFound { path: path.clone() })?;

        node.permissions = perm;
        node.modified = SystemTime::now();

        Ok(())
    }
}
```

### FsSync

```rust
pub trait FsSync: Send + Sync {
    /// Sync all pending changes to persistent storage.
    fn sync(&self) -> Result<(), FsError>;

    /// Sync a specific file's data and metadata to storage.
    fn fsync(&self, path: &Path) -> Result<(), FsError>;
}
```

Implementation:

```rust
impl FsSync for TutorialFs {
    fn sync(&self) -> Result<(), FsError> {
        // In-memory: nothing to sync
        Ok(())
    }

    fn fsync(&self, path: &Path) -> Result<(), FsError> {
        let path = Self::normalize_path(path);
        let inner = self.inner.read().unwrap();

        // Verify file exists
        if !inner.nodes.contains_key(&path) {
            return Err(FsError::NotFound { path });
        }

        // In-memory: nothing to sync
        Ok(())
    }
}
```

For a real disk-backed filesystem, you'd call `fsync()` or equivalent.

### FsStats

```rust
pub trait FsStats: Send + Sync {
    /// Get filesystem statistics.
    fn statfs(&self) -> Result<StatFs, FsError>;
}

pub struct StatFs {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub total_inodes: Option<u64>,
    pub available_inodes: Option<u64>,
    pub used_inodes: Option<u64>,
    pub block_size: Option<u64>,
}
```

Implementation:

```rust
impl FsStats for TutorialFs {
    fn statfs(&self) -> Result<StatFs, FsError> {
        let inner = self.inner.read().unwrap();

        // Calculate used space
        let used_bytes: u64 = inner.nodes.values()
            .map(|n| n.content.len() as u64)
            .sum();

        let used_inodes = inner.nodes.len() as u64;

        Ok(StatFs {
            total_bytes: inner.total_size,
            available_bytes: inner.total_size.saturating_sub(used_bytes),
            used_bytes,
            total_inodes: Some(1_000_000),
            available_inodes: Some(1_000_000 - used_inodes),
            used_inodes: Some(used_inodes),
            block_size: Some(4096),
        })
    }
}
```

## Verify FsFull

After implementing all component traits:

```rust
fn use_fs_full<B: FsFull>(_: &B) {}

fn main() {
    let fs = TutorialFs::new();
    use_fs_full(&fs);  // ✅ Compiles!
}
```

## Using FsFull

```rust
use anyfs_backend::{FsFull, FsError, Permissions};

/// Make a file read-only.
fn make_readonly<B: FsFull>(fs: &B, path: &Path) -> Result<(), FsError> {
    fs.set_permissions(path, Permissions::from_mode(0o444))
}

/// Get disk usage percentage.
fn disk_usage<B: FsFull>(fs: &B) -> Result<f64, FsError> {
    let stats = fs.statfs()?;
    Ok((stats.used_bytes as f64 / stats.total_bytes as f64) * 100.0)
}

/// Write and flush immediately.
fn write_sync<B: FsFull>(fs: &B, path: &Path, data: &[u8]) -> Result<(), FsError> {
    fs.write(path, data)?;
    fs.sync(path)
}
```

## Testing

```rust
#[test]
fn test_set_permissions() {
    let fs = TutorialFs::new();
    
    fs.write(Path::new("/file.txt"), b"data").unwrap();
    fs.set_permissions(Path::new("/file.txt"), Permissions::from_mode(0o600)).unwrap();
    
    let meta = fs.metadata(Path::new("/file.txt")).unwrap();
    assert_eq!(meta.permissions.mode(), 0o600);
}

#[test]
fn test_statfs() {
    let fs = TutorialFs::new();
    
    // Write some data
    fs.write(Path::new("/a.txt"), b"Hello").unwrap();
    fs.write(Path::new("/b.txt"), b"World!").unwrap();
    
    let stats = fs.statfs().unwrap();
    assert!(stats.used_bytes >= 11);  // At least "Hello" + "World!"
    assert!(stats.available_bytes < stats.total_bytes);
}
```

## Summary

`FsFull = Fs + FsLink + FsPermissions + FsSync + FsStats`

| Trait           | Purpose                |
| --------------- | ---------------------- |
| `FsPermissions` | Set mode and ownership |
| `FsSync`        | Flush data to storage  |
| `FsStats`       | Disk space information |

This is suitable for most real-world applications.

Next: [FsInode: FUSE Support →](./08-fs-inode.md)
