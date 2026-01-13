//! # anyfs-backend
//!
//! Core traits and types for the AnyFS pluggable virtual filesystem standard.
//!
//! This crate defines the trait hierarchy that backends implement:
//!
//! - **Layer 1 (Core):** `FsRead`, `FsWrite`, `FsDir` → `Fs`
//! - **Layer 2 (Extended):** `FsLink`, `FsPermissions`, `FsSync`, `FsStats` → `FsFull`
//! - **Layer 3 (FUSE):** `FsInode` → `FsFuse`
//! - **Layer 4 (POSIX):** `FsHandles`, `FsLock`, `FsXattr` → `FsPosix`
//!
//! ## Quick Start
//!
//! Most users only need `Fs`:
//!
//! ```rust,ignore
//! use anyfs_backend::{Fs, FsError};
//!
//! fn process<B: Fs>(backend: &B) -> Result<(), FsError> {
//!     let data = backend.read(std::path::Path::new("/input.txt"))?;
//!     backend.write(std::path::Path::new("/output.txt"), &data)?;
//!     Ok(())
//! }
//! ```

// Private modules
mod error;
mod path_resolver;
mod traits;
mod types;

// TODO: Add these modules as they are implemented
// mod layer;
// mod ext;
// mod markers;

// Public re-exports - error types
pub use error::FsError;

// Public re-exports - core types
pub use types::{
    DirEntry, FileType, Handle, LockType, Metadata, OpenFlags, Permissions, ROOT_INODE, StatFs,
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
