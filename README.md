# anyfs-backend

[![CI](https://github.com/DK26/anyfs-backend/actions/workflows/ci.yml/badge.svg)](https://github.com/DK26/anyfs-backend/actions/workflows/ci.yml)
[![Security Audit](https://github.com/DK26/anyfs-backend/actions/workflows/ci.yml/badge.svg?label=audit)](https://github.com/DK26/anyfs-backend/security)
[![Crates.io](https://img.shields.io/crates/v/anyfs-backend.svg)](https://crates.io/crates/anyfs-backend)
[![Downloads](https://img.shields.io/crates/d/anyfs-backend.svg)](https://crates.io/crates/anyfs-backend)
[![Documentation](https://docs.rs/anyfs-backend/badge.svg)](https://docs.rs/anyfs-backend)
[![Rust 1.68+](https://img.shields.io/badge/rust-1.68%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

Core traits and types for the **AnyFS pluggable virtual filesystem standard**.

This crate defines the trait hierarchy that filesystem backends implement. It contains **only trait definitions and types** — no concrete implementations. For backends, see the [`anyfs`](https://crates.io/crates/anyfs) crate.

📖 **[User Guide](https://dk26.github.io/anyfs-backend/)** · 📐 **[Design Manual](https://dk26.github.io/anyfs-design-manual/)**

---

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
anyfs-backend = "0.1"
```

### Using a Backend

Most users only need the `Fs` trait — it covers 90% of use cases:

```rust
use anyfs_backend::{Fs, FsError};
use std::path::Path;

// This function works with ANY backend that implements Fs
fn process_files<B: Fs>(backend: &B) -> Result<(), FsError> {
    // Read a file
    let data = backend.read(Path::new("/config.json"))?;
    
    // Write a file
    backend.write(Path::new("/output.txt"), b"Hello, world!")?;
    
    // Create directories
    backend.create_dir_all(Path::new("/logs/2024/01"))?;
    
    // List directory contents
    for entry in backend.read_dir(Path::new("/"))? {
        let entry = entry?;
        println!("{} ({:?})", entry.name, entry.file_type);
    }
    
    // Check existence
    if backend.exists(Path::new("/config.json"))? {
        println!("Config exists!");
    }
    
    Ok(())
}
```

### Implementing a Backend

To create your own backend, implement the component traits:

```rust
use anyfs_backend::{FsRead, FsWrite, FsDir, Fs, FsError, Metadata, ReadDirIter};
use std::path::Path;
use std::io::{Read, Write};

struct MyBackend {
    // Your state here
}

impl FsRead for MyBackend {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        // Your implementation
        todo!()
    }
    
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        String::from_utf8(self.read(path)?).map_err(|_| FsError::InvalidData {
            path: path.to_path_buf(),
            details: "not valid UTF-8".into(),
        })
    }
    
    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        todo!()
    }
    
    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        todo!()
    }
    
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        todo!()
    }
    
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
        todo!()
    }
}

impl FsWrite for MyBackend {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        todo!()
    }
    
    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        todo!()
    }
    
    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> {
        todo!()
    }
    
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        todo!()
    }
    
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        todo!()
    }
    
    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        todo!()
    }
    
    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        todo!()
    }
}

impl FsDir for MyBackend {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        todo!()
    }
    
    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        todo!()
    }
    
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        todo!()
    }
    
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        todo!()
    }
    
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        todo!()
    }
}

// Now MyBackend automatically implements Fs!
// fn use_backend(backend: &impl Fs) { ... }
```

---

## Trait Hierarchy

AnyFS uses a **layered trait architecture**. Each layer builds on the previous:

```text
Layer 1 (Core):     FsRead + FsWrite + FsDir = Fs
                                              ↓
Layer 2 (Extended): Fs + FsLink + FsPermissions + FsSync + FsStats = FsFull
                                              ↓
Layer 3 (FUSE):     FsFull + FsInode = FsFuse
                                              ↓
Layer 4 (POSIX):    FsFuse + FsHandles + FsLock + FsXattr = FsPosix
```

### Which Trait Should I Use?

| Trait         | Use Case                                      | Coverage              |
| ------------- | --------------------------------------------- | --------------------- |
| **`Fs`**      | Basic file I/O, config files, data processing | 90% of apps           |
| **`FsFull`**  | Backup tools, file managers, symlinks         | Extended features     |
| **`FsFuse`**  | FUSE mounts, virtual drives                   | Userspace filesystems |
| **`FsPosix`** | Databases, file locking, xattrs               | Full POSIX semantics  |

### Complete Trait Reference

#### Component Traits (What You Implement)

| Trait           | Provides              | Key Methods                                    | When to Use                            |
| --------------- | --------------------- | ---------------------------------------------- | -------------------------------------- |
| `FsRead`        | Read operations       | `read`, `read_to_string`, `exists`, `metadata` | Always (core requirement)              |
| `FsWrite`       | Write operations      | `write`, `append`, `remove_file`, `rename`     | Always (core requirement)              |
| `FsDir`         | Directory operations  | `read_dir`, `create_dir`, `remove_dir_all`     | Always (core requirement)              |
| `FsLink`        | Symbolic/hard links   | `symlink`, `hard_link`, `read_link`            | Backup tools, Unix-like behavior       |
| `FsPermissions` | Permission management | `set_permissions`                              | File managers, security-aware apps     |
| `FsSync`        | Force disk sync       | `sync`, `sync_path`                            | Databases, crash-safe writes           |
| `FsStats`       | Filesystem statistics | `statfs`                                       | Disk space monitoring, quotas          |
| `FsInode`       | Inode ↔ path mapping  | `path_to_inode`, `inode_to_path`, `lookup`     | FUSE filesystems                       |
| `FsHandles`     | Open file handles     | `open`, `close`, `read_at`, `write_at`         | Random access, concurrent file access  |
| `FsLock`        | File locking          | `lock`, `unlock`, `try_lock`                   | Multi-process coordination, databases  |
| `FsXattr`       | Extended attributes   | `get_xattr`, `set_xattr`, `list_xattr`         | Metadata storage, macOS/Linux features |
| `FsPath`        | Path canonicalization | `canonicalize`, `soft_canonicalize`            | Symlink resolution, path normalization |

#### Composite Traits (What You Use in Bounds)

| Trait     | Combines                                         | Typical Consumer                        |
| --------- | ------------------------------------------------ | --------------------------------------- |
| `Fs`      | `FsRead + FsWrite + FsDir`                       | Generic file processing code            |
| `FsFull`  | `Fs + FsLink + FsPermissions + FsSync + FsStats` | File managers, backup/restore tools     |
| `FsFuse`  | `FsFull + FsInode`                               | FUSE filesystem implementations         |
| `FsPosix` | `FsFuse + FsHandles + FsLock + FsXattr`          | Databases, POSIX-compliant applications |

### Blanket Implementations

All composite traits have **blanket implementations**. Just implement the component traits and you get the composite for free:

```rust
// Implement FsRead, FsWrite, FsDir → get Fs automatically
// Implement Fs, FsLink, FsPermissions, FsSync, FsStats → get FsFull automatically
```

---

## Core Types

| Type          | Description                                              |
| ------------- | -------------------------------------------------------- |
| `Metadata`    | File metadata: size, type, times, permissions, inode     |
| `FileType`    | `File`, `Directory`, or `Symlink`                        |
| `DirEntry`    | Single entry from directory listing                      |
| `Permissions` | Unix-style permission bits (0o755, etc.)                 |
| `StatFs`      | Filesystem statistics (total/used/available space)       |
| `Handle`      | Opaque file handle for POSIX operations                  |
| `OpenFlags`   | Flags for opening files (READ, WRITE, CREATE, etc.)      |
| `LockType`    | `Shared` or `Exclusive` file lock                        |
| `FsError`     | Comprehensive error type with path and operation context |

---

## Error Handling

All operations return `Result<T, FsError>`. Errors include context:

```rust
use anyfs_backend::FsError;
use std::path::PathBuf;

// Errors tell you what went wrong and where
let err = FsError::NotFound { path: PathBuf::from("/missing.txt") };
println!("{}", err); // "not found: /missing.txt"

let err = FsError::PermissionDenied { 
    path: PathBuf::from("/secret"), 
    operation: "read" 
};
println!("{}", err); // "read: permission denied: /secret"
```

---

## Middleware (Layer Trait)

Use the `Layer` trait for Tower-style middleware composition:

```rust
use anyfs_backend::Layer;

// Configuration
struct CacheConfig { max_entries: usize }

// Middleware wrapper
struct CacheMiddleware<B> {
    inner: B,
    config: CacheConfig,
}

// Layer creates the middleware
struct CacheLayer { config: CacheConfig }

impl<B> Layer<B> for CacheLayer {
    type Backend = CacheMiddleware<B>;
    
    fn layer(self, backend: B) -> Self::Backend {
        CacheMiddleware { inner: backend, config: self.config }
    }
}
```

Chain middleware with `LayerExt`:

```rust
use anyfs_backend::LayerExt;

// backend.layer(CacheLayer::new()).layer(LoggingLayer::new())
```

---

## Extension Traits

### FsExt

Convenience methods for any `Fs` backend:

```rust
use anyfs_backend::{Fs, FsExt};
use std::path::Path;

fn check<B: Fs>(backend: &B) {
    // Type checking
    let _ = backend.is_file(Path::new("/foo"));    // Ok(bool)
    let _ = backend.is_dir(Path::new("/bar"));     // Ok(bool)
    let _ = backend.file_size(Path::new("/baz"));  // Ok(u64)
}
```

### FsExtJson (requires `serde` feature)

```toml
[dependencies]
anyfs-backend = { version = "0.1", features = ["serde"] }
```

```rust
use anyfs_backend::{Fs, FsExtJson};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Config { name: String }

fn save_config<B: Fs>(backend: &B, config: &Config) {
    backend.write_json(Path::new("/config.json"), config).unwrap();
}

fn load_config<B: Fs>(backend: &B) -> Config {
    backend.read_json(Path::new("/config.json")).unwrap()
}
```

---

## Thread Safety

All traits require `Send + Sync`. Methods take `&self` (not `&mut self`), enabling safe concurrent access:

```rust
use std::sync::Arc;
use std::thread;

fn concurrent_access<B: anyfs_backend::Fs + 'static>(backend: Arc<B>) {
    let handles: Vec<_> = (0..4).map(|i| {
        let backend = Arc::clone(&backend);
        thread::spawn(move || {
            backend.read(std::path::Path::new("/shared.txt")).ok();
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

---

## Feature Flags

| Feature | Description                                        |
| ------- | -------------------------------------------------- |
| `serde` | Enable serialization for types + `FsExtJson` trait |

---

## Related Crates

| Crate                                               | Description                                        |
| --------------------------------------------------- | -------------------------------------------------- |
| [`anyfs`](https://crates.io/crates/anyfs)           | Concrete backends: Memory, Native, Overlay, SQLite |
| [`anyfs-fuse`](https://crates.io/crates/anyfs-fuse) | FUSE integration for mounting backends             |

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

See [AGENTS.md](AGENTS.md) for contribution guidelines and architecture decisions.

## For AI Agents

See [LLM_CONTEXT.md](LLM_CONTEXT.md) for a Context7-style reference optimized for LLM consumption.
