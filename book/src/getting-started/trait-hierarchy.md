# Trait Hierarchy

The anyfs-backend crate organizes filesystem operations into a layered hierarchy of traits. Each layer adds more capabilities.

## The Four Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 4: FsPosix                                           │
│  Full POSIX semantics with handles and locks                │
│  = FsFuse + FsHandles + FsLock + FsXattr                    │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: FsFuse                                            │
│  FUSE-compatible with inode operations                      │
│  = FsFull + FsInode                                         │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: FsFull                                            │
│  Complete filesystem with links, permissions, stats         │
│  = Fs + FsLink + FsPermissions + FsSync + FsStats           │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Fs                                                │
│  Basic file operations                                      │
│  = FsRead + FsWrite + FsDir                                 │
└─────────────────────────────────────────────────────────────┘
```

## Layer 1: Fs (Basic Operations)

The minimum for a functional filesystem.

| Trait     | Purpose                                   |
| --------- | ----------------------------------------- |
| `FsRead`  | Read files, get metadata, check existence |
| `FsWrite` | Write files, delete files                 |
| `FsDir`   | List, create, remove directories; rename  |

**Use `Fs` when:** You need basic file I/O and don't care about permissions, symlinks, or advanced features.

```rust
fn backup<B: Fs>(fs: &B, src: &Path, dst: &Path) -> Result<(), FsError> {
    let data = fs.read(src)?;
    fs.write(dst, &data)
}
```

## Layer 2: FsFull (Complete Filesystem)

Adds features most real filesystems need.

| Trait           | Purpose                             |
| --------------- | ----------------------------------- |
| `FsLink`        | Symbolic and hard links             |
| `FsPermissions` | Set permissions and ownership       |
| `FsSync`        | Flush writes to storage             |
| `FsStats`       | Filesystem statistics (space usage) |

**Use `FsFull` when:** You need symlinks, permissions, or disk usage stats.

```rust
fn create_symlink<B: FsFull>(fs: &B, target: &Path, link: &Path) -> Result<(), FsError> {
    fs.symlink(target, link)
}
```

## Layer 3: FsFuse (FUSE Support)

Adds inode-based operations for FUSE implementations.

| Trait     | Purpose                                |
| --------- | -------------------------------------- |
| `FsInode` | Path↔inode conversion, lookup by inode |

**Use `FsFuse` when:** Building a FUSE filesystem that operates on inodes.

```rust
fn get_child<B: FsFuse>(fs: &B, parent_inode: u64, name: &str) -> Result<u64, FsError> {
    fs.lookup(parent_inode, name)
}
```

## Layer 4: FsPosix (Full POSIX)

Complete POSIX semantics.

| Trait       | Purpose                          |
| ----------- | -------------------------------- |
| `FsHandles` | Open files, read/write at offset |
| `FsLock`    | File locking (shared/exclusive)  |
| `FsXattr`   | Extended attributes              |

**Use `FsPosix` when:** You need file handles, locking, or extended attributes.

```rust
fn locked_write<B: FsPosix>(fs: &B, path: &Path, data: &[u8]) -> Result<(), FsError> {
    let handle = fs.open(path, OpenFlags::WRITE | OpenFlags::CREATE)?;
    fs.lock(handle, LockType::Exclusive)?;
    fs.write_at(handle, 0, data)?;
    fs.unlock(handle)?;
    fs.close(handle)
}
```

## Choosing the Right Trait Bound

| Your needs                     | Use this bound |
| ------------------------------ | -------------- |
| Basic read/write/list          | `Fs`           |
| + symlinks, permissions, stats | `FsFull`       |
| + inode operations (FUSE)      | `FsFuse`       |
| + file handles, locking        | `FsPosix`      |

## Automatic Implementation

The composite traits (`Fs`, `FsFull`, `FsFuse`, `FsPosix`) are automatically implemented via blanket impls. You only implement the component traits:

```rust
// You implement these:
impl FsRead for MyBackend { ... }
impl FsWrite for MyBackend { ... }
impl FsDir for MyBackend { ... }

// This is automatic:
// impl Fs for MyBackend {}  // ← Provided by blanket impl
```

## Thread Safety

All traits require `Send + Sync`. This means:

- Methods take `&self`, not `&mut self`
- Use interior mutability (`RwLock`, `Mutex`) for mutable state
- Safe for concurrent access from multiple threads

```rust
pub struct MyBackend {
    // Use RwLock for mutable state
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
}

impl FsRead for MyBackend {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        // Acquire read lock
        self.files.read().unwrap().get(path).cloned()
            .ok_or_else(|| FsError::NotFound { path: path.to_path_buf() })
    }
}
```
