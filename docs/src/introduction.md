# AnyFS Backend Guide

**anyfs-backend** is the foundational crate for the AnyFS ecosystem. It provides a trait-based abstraction for filesystem operations, allowing you to:

- Write code that works with any filesystem backend
- Create middleware layers that add cross-cutting functionality
- Build FUSE filesystems with a clean, type-safe API

## Who Is This Guide For?

- **Backend implementers**: You want to create a new filesystem backend (S3, Google Drive, in-memory, etc.)
- **Middleware authors**: You want to add logging, caching, encryption, or other features as composable layers
- **Library users**: You want to understand how to use anyfs effectively

## What You'll Learn

### [Implementing a Backend](./tutorials/backend/README.md)

Step-by-step guide to implementing a complete filesystem backend, from basic file operations to full POSIX support.

### [Implementing Middleware](./tutorials/middleware/README.md)

How to create reusable middleware layers using the Tower-inspired Layer pattern.

## Quick Example

```rust
use anyfs_backend::{Fs, FsError};
use std::path::Path;

// Write a generic function that works with ANY backend
fn copy_file<B: Fs>(fs: &B, src: &Path, dst: &Path) -> Result<(), FsError> {
    let content = fs.read(src)?;
    fs.write(dst, &content)?;
    Ok(())
}

// Use with any backend that implements Fs
fn main() {
    let fs = my_backend::MyFs::new();
    copy_file(&fs, Path::new("/src.txt"), Path::new("/dst.txt")).unwrap();
}
```

## Design Principles

1. **Trait-based**: All operations are defined as traits, enabling generic code
2. **Layered**: Traits are organized in layers (Fs → FsFull → FsFuse → FsPosix)
3. **Composable**: Middleware layers can be stacked to add functionality
4. **Thread-safe**: All traits require `Send + Sync` for concurrent access
5. **Error-rich**: Detailed error types with full context
