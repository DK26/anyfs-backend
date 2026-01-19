//! # anyfs-backend
//!
//! Core traits and types for the **AnyFS pluggable virtual filesystem standard**.
//!
//! This crate provides the foundational API that filesystem backends implement.
//! It contains **only trait definitions and types** — no concrete implementations.
//! Implementations live in the `anyfs` crate.
//!
//! ---
//!
//! ## Quick Start
//!
//! Most users only need [`Fs`] — it covers 90% of use cases.
//!
//! A typical usage pattern with any backend that implements `Fs`:
//!
//! ```rust
//! use anyfs_backend::Fs;
//! use std::path::Path;
//!
//! // Generic function that works with any Fs implementation
//! fn work_with_files<B: Fs>(backend: &B) -> Result<(), anyfs_backend::FsError> {
//!     let data = backend.read(Path::new("/input.txt"))?;
//!     backend.write(Path::new("/output.txt"), &data)?;
//!     backend.create_dir_all(Path::new("/archive/2024"))?;
//!     for entry in backend.read_dir(Path::new("/"))? {
//!         println!("{}", entry?.name);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! See the `anyfs` crate for concrete backend implementations.
//!
//! ---
//!
//! ## Core Types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Fs`] | Basic filesystem trait — read, write, and directory operations |
//! | [`FsFull`] | Extended filesystem — adds links, permissions, sync, stats |
//! | [`FsFuse`] | FUSE-mountable — adds inode-based operations |
//! | [`FsPosix`] | Full POSIX — adds handles, locks, extended attributes |
//! | [`FsError`] | Comprehensive error type with context |
//! | [`Metadata`] | File/directory metadata (size, type, times, permissions) |
//! | [`DirEntry`] | Single directory listing entry |
//!
//! ---
//!
//! ## Which Trait Should I Use?
//!
//! **[`Fs`]** — When you need basic file operations.
//! - Use for: Config files, data serialization, file processing, simple I/O
//! - Methods: `read`, `write`, `create_dir`, `read_dir`, `exists`, `metadata`
//! - Coverage: **90% of use cases**
//!
//! **[`FsFull`]** — When you need filesystem features beyond basic I/O.
//! - Use for: Backup tools, file managers, archive extraction
//! - Adds: `symlink`, `hard_link`, `set_permissions`, `sync`, `statfs`
//! - Includes: Everything in [`Fs`]
//!
//! **[`FsFuse`]** — When building a FUSE filesystem.
//! - Use for: Userspace mounts, virtual drives, network filesystems
//! - Adds: `path_to_inode`, `inode_to_path`, `lookup`, `metadata_by_inode`
//! - Includes: Everything in [`FsFull`]
//!
//! **[`FsPosix`]** — When you need full POSIX semantics.
//! - Use for: Database storage, lock-based coordination, xattr metadata
//! - Adds: `open`/`close` handles, `lock`/`unlock`, `get_xattr`/`set_xattr`
//! - Includes: Everything in [`FsFuse`]
//!
//! ---
//!
//! ## Trait Hierarchy
//!
//! ```text
//! Layer 1 (Core):     FsRead + FsWrite + FsDir = Fs
//!                                               ↓
//! Layer 2 (Extended): Fs + FsLink + FsPermissions + FsSync + FsStats = FsFull
//!                                               ↓
//! Layer 3 (FUSE):     FsFull + FsInode = FsFuse
//!                                               ↓
//! Layer 4 (POSIX):    FsFuse + FsHandles + FsLock + FsXattr = FsPosix
//! ```
//!
//! All composite traits ([`Fs`], [`FsFull`], [`FsFuse`], [`FsPosix`]) have **blanket
//! implementations**. Just implement the component traits and you get the composite
//! trait for free.
//!
//! ---
//!
//! ## Error Handling
//!
//! All operations return `Result<T, FsError>`. Errors include context:
//!
//! ```rust
//! use anyfs_backend::FsError;
//! use std::path::PathBuf;
//!
//! // Errors include the path that caused the problem
//! let err = FsError::NotFound { path: PathBuf::from("/missing.txt") };
//! assert_eq!(err.to_string(), "not found: /missing.txt");
//!
//! // Permission errors include the operation
//! let err = FsError::PermissionDenied {
//!     path: PathBuf::from("/secret"),
//!     operation: "read",
//! };
//! assert_eq!(err.to_string(), "read: permission denied: /secret");
//! ```
//!
//! ---
//!
//! ## Thread Safety
//!
//! All traits require `Send + Sync`. Methods take `&self` (not `&mut self`),
//! enabling safe concurrent access. Backends use interior mutability internally.
//!
//! You can safely share a backend across threads using `Arc<B>` and spawn
//! concurrent operations without explicit locking at the call site.
//!
//! ---
//!
//! ## Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `serde` | Enable serialization for [`Metadata`], [`DirEntry`], [`Permissions`], etc. |
//!
//! ---
//!
//! ## Crate Organization
//!
//! This crate (`anyfs-backend`) contains **only traits and types**.
//!
//! For concrete implementations, see the `anyfs` crate which provides:
//! - `MemoryBackend` — In-memory filesystem
//! - `NativeBackend` — Wrapper around `std::fs`
//! - `OverlayBackend` — UnionFS-style layering
//! - `FileStorage` — Type-erased filesystem wrapper
//! - Middleware (encryption, compression, caching, etc.)

// Private modules
mod error;
mod ext;
mod layer;
mod markers;
mod path_resolver;
mod traits;
mod types;

// Public re-exports - error types
pub use error::FsError;

// Public re-exports - core types
pub use types::{
    DirEntry, FileType, Handle, LockType, Metadata, OpenFlags, Permissions, StatFs, ROOT_INODE,
};

// Public re-exports - Layer 1 core traits
pub use traits::{Fs, FsDir, FsRead, FsWrite, ReadDirIter};

// Public re-exports - Layer 2 extended traits
pub use traits::{FsFull, FsLink, FsPath, FsPermissions, FsStats, FsSync};

// Public re-exports - Layer 3 FUSE traits
pub use traits::{FsFuse, FsInode};

// Public re-exports - Layer 4 POSIX traits
pub use traits::{FsHandles, FsLock, FsPosix, FsXattr};

// Public re-exports - path resolution
pub use path_resolver::PathResolver;

// Public re-exports - infrastructure
pub use ext::FsExt;
pub use layer::{Layer, LayerExt};
pub use markers::SelfResolving;

// Conditional re-exports
#[cfg(feature = "serde")]
pub use ext::FsExtJson;
