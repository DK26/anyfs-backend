//! # Filesystem Traits
//!
//! The core trait hierarchy that defines the AnyFS interface.
//!
//! ## Trait Layers
//!
//! AnyFS uses a layered trait architecture. Each layer builds on the previous,
//! allowing backends to implement only the features they support:
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
//! ## Quick Reference
//!
//! | Layer | Composite Trait | Component Traits | Use Case |
//! |-------|-----------------|------------------|----------|
//! | 1 | [`Fs`] | [`FsRead`], [`FsWrite`], [`FsDir`] | Basic file I/O (90% of uses) |
//! | 2 | [`FsFull`] | + [`FsLink`], [`FsPermissions`], [`FsSync`], [`FsStats`] | Full `std::fs` features |
//! | 3 | [`FsFuse`] | + [`FsInode`] | FUSE mounting |
//! | 4 | [`FsPosix`] | + [`FsHandles`], [`FsLock`], [`FsXattr`] | Full POSIX semantics |
//!
//! ## Blanket Implementations
//!
//! All composite traits have blanket implementations. Implement the component
//! traits, and you get the composite trait automatically:
//!
//! ```rust
//! use anyfs_backend::{Fs, FsRead, FsWrite, FsDir, ReadDirIter};
//!
//! // Define a backend
//! struct MyBackend;
//!
//! // Implement component traits (stubs shown)
//! # impl FsRead for MyBackend {
//! #     fn read(&self, _: &std::path::Path) -> Result<Vec<u8>, anyfs_backend::FsError> { Ok(vec![]) }
//! #     fn read_to_string(&self, _: &std::path::Path) -> Result<String, anyfs_backend::FsError> { Ok(String::new()) }
//! #     fn read_range(&self, _: &std::path::Path, _: u64, _: usize) -> Result<Vec<u8>, anyfs_backend::FsError> { Ok(vec![]) }
//! #     fn exists(&self, _: &std::path::Path) -> Result<bool, anyfs_backend::FsError> { Ok(true) }
//! #     fn metadata(&self, _: &std::path::Path) -> Result<anyfs_backend::Metadata, anyfs_backend::FsError> { Ok(anyfs_backend::Metadata::default()) }
//! #     fn open_read(&self, _: &std::path::Path) -> Result<Box<dyn std::io::Read + Send>, anyfs_backend::FsError> { unimplemented!() }
//! # }
//! # impl FsWrite for MyBackend {
//! #     fn write(&self, _: &std::path::Path, _: &[u8]) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn append(&self, _: &std::path::Path, _: &[u8]) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn truncate(&self, _: &std::path::Path, _: u64) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn remove_file(&self, _: &std::path::Path) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn rename(&self, _: &std::path::Path, _: &std::path::Path) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn copy(&self, _: &std::path::Path, _: &std::path::Path) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn open_write(&self, _: &std::path::Path) -> Result<Box<dyn std::io::Write + Send>, anyfs_backend::FsError> { unimplemented!() }
//! # }
//! # impl FsDir for MyBackend {
//! #     fn read_dir(&self, _: &std::path::Path) -> Result<ReadDirIter, anyfs_backend::FsError> { Ok(ReadDirIter::from_vec(vec![])) }
//! #     fn create_dir(&self, _: &std::path::Path) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn create_dir_all(&self, _: &std::path::Path) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn remove_dir(&self, _: &std::path::Path) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! #     fn remove_dir_all(&self, _: &std::path::Path) -> Result<(), anyfs_backend::FsError> { Ok(()) }
//! # }
//!
//! // Now MyBackend automatically implements Fs!
//! fn use_fs<B: Fs>(_backend: &B) { /* ... */ }
//! let my_backend = MyBackend;
//! use_fs(&my_backend); // ✓ Works
//! ```
//!
//! ## Thread Safety
//!
//! All traits require `Send + Sync`. Methods take `&self` to enable concurrent
//! access. Backends use interior mutability for thread-safe state management.
//!
//! ## Object Safety
//!
//! All traits are object-safe and can be used as trait objects:
//!
//! ```rust
//! use anyfs_backend::Fs;
//!
//! fn process(fs: &dyn Fs) {
//!     let _ = fs.read(std::path::Path::new("/file.txt"));
//! }
//! ```

mod fs_dir;
mod fs_handles;
mod fs_inode;
mod fs_link;
mod fs_lock;
mod fs_path;
mod fs_permissions;
mod fs_read;
mod fs_stats;
mod fs_sync;
mod fs_write;
mod fs_xattr;

// Layer 1 - Core traits
pub use fs_dir::{FsDir, ReadDirIter};
pub use fs_read::FsRead;
pub use fs_write::FsWrite;

// Layer 2 - Extended traits
pub use fs_link::FsLink;
pub use fs_path::FsPath;
pub use fs_permissions::FsPermissions;
pub use fs_stats::FsStats;
pub use fs_sync::FsSync;

// Layer 3 - FUSE traits
pub use fs_inode::FsInode;

// Layer 4 - POSIX traits
pub use fs_handles::FsHandles;
pub use fs_lock::FsLock;
pub use fs_xattr::FsXattr;

/// Basic filesystem — covers 90% of use cases.
///
/// The primary trait for filesystem operations. Combines reading ([`FsRead`]),
/// writing ([`FsWrite`]), and directory operations ([`FsDir`]).
///
/// # When to Use
///
/// Use `Fs` when you need:
/// - Read/write file contents
/// - Create/remove files and directories
/// - List directory contents
/// - Check if paths exist
/// - Get file metadata
///
/// # Blanket Implementation
///
/// Automatically implemented for any type that implements all three component traits.
/// You never need to implement `Fs` directly — just implement the components.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{Fs, FsError};
/// use std::path::Path;
///
/// // Generic function that works with any Fs implementation
/// fn backup_file<B: Fs>(fs: &B, src: &Path, dst: &Path) -> Result<(), FsError> {
///     // Read source file
///     let data = fs.read(src)?;
///     
///     // Ensure destination directory exists
///     if let Some(parent) = dst.parent() {
///         fs.create_dir_all(parent)?;
///     }
///     
///     // Write to destination
///     fs.write(dst, &data)?;
///     
///     Ok(())
/// }
/// ```
///
/// # Available Methods
///
/// From [`FsRead`]:
/// - `read`, `read_to_string`, `read_range`
/// - `exists`, `metadata`, `open_read`
///
/// From [`FsWrite`]:
/// - `write`, `append`, `truncate`
/// - `remove_file`, `rename`, `copy`, `open_write`
///
/// From [`FsDir`]:
/// - `read_dir`, `create_dir`, `create_dir_all`
/// - `remove_dir`, `remove_dir_all`
pub trait Fs: FsRead + FsWrite + FsDir {}

// Blanket implementation - any type implementing all three gets Fs for free
impl<T: FsRead + FsWrite + FsDir> Fs for T {}

/// Full filesystem with all `std::fs` features.
///
/// Extends [`Fs`] with links, permissions, synchronization, and statistics.
///
/// # When to Use
///
/// Use `FsFull` when you need:
/// - Symbolic links or hard links ([`FsLink`])
/// - Permission management ([`FsPermissions`])
/// - Force writes to disk ([`FsSync`])
/// - Filesystem capacity information ([`FsStats`])
///
/// # Blanket Implementation
///
/// Automatically implemented for any type implementing `Fs + FsLink + FsPermissions + FsSync + FsStats`.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsFull, FsError, Permissions};
/// use std::path::Path;
///
/// // Generic function that works with any FsFull implementation
/// fn create_backup<B: FsFull>(fs: &B) -> Result<(), FsError> {
///     // Write the main file
///     fs.write(Path::new("/data/config.json"), b"{}")?;
///     
///     // Create a hard link as backup
///     fs.hard_link(Path::new("/data/config.json"), Path::new("/backups/config.json"))?;
///     
///     // Make backup read-only
///     fs.set_permissions(Path::new("/backups/config.json"), Permissions::from_mode(0o444))?;
///     
///     // Ensure changes are on disk
///     fs.sync()?;
///     
///     // Check available space
///     let stats = fs.statfs()?;
///     println!("Available: {} bytes", stats.available_bytes);
///     
///     Ok(())
/// }
/// ```
///
/// # Additional Methods
///
/// From [`FsLink`]:
/// - `symlink`, `hard_link`, `read_link`, `symlink_metadata`
///
/// From [`FsPermissions`]:
/// - `set_permissions`
///
/// From [`FsSync`]:
/// - `sync`, `fsync`
///
/// From [`FsStats`]:
/// - `statfs`
pub trait FsFull: Fs + FsLink + FsPermissions + FsSync + FsStats {}

// Blanket implementation
impl<T: Fs + FsLink + FsPermissions + FsSync + FsStats> FsFull for T {}

/// FUSE-mountable filesystem.
///
/// Extends [`FsFull`] with inode-based operations required for FUSE mounting.
///
/// # When to Use
///
/// Use `FsFuse` when you need:
/// - Path-to-inode mapping ([`FsInode::path_to_inode`])
/// - Inode-to-path reverse lookup ([`FsInode::inode_to_path`])
/// - Directory entry lookup by name ([`FsInode::lookup`])
/// - Metadata retrieval by inode ([`FsInode::metadata_by_inode`])
///
/// # FUSE Integration
///
/// FUSE (Filesystem in Userspace) operates primarily with inodes rather than
/// paths. This trait provides the bridge between path-based and inode-based
/// operations.
///
/// # Blanket Implementation
///
/// Automatically implemented for any type implementing `FsFull + FsInode`.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsFuse, FsError, ROOT_INODE};
/// use std::ffi::OsStr;
///
/// // Generic function that works with any FsFuse implementation
/// fn fuse_lookup<B: FsFuse>(fs: &B, name: &str) -> Result<u64, FsError> {
///     // Start from root inode
///     let root_inode = ROOT_INODE;  // Always 1
///     
///     // Look up child by name
///     let child_inode = fs.lookup(root_inode, OsStr::new(name))?;
///     
///     // Get metadata by inode
///     let meta = fs.metadata_by_inode(child_inode)?;
///     println!("Found {} ({:?}, {} bytes)", name, meta.file_type, meta.size);
///     
///     Ok(child_inode)
/// }
/// ```
///
/// # Additional Methods
///
/// From [`FsInode`]:
/// - `path_to_inode` — Convert path to inode number
/// - `inode_to_path` — Convert inode back to path
/// - `lookup` — Find child inode by name within a directory
/// - `metadata_by_inode` — Get metadata without path lookup
pub trait FsFuse: FsFull + FsInode {}

// Blanket implementation
impl<T: FsFull + FsInode> FsFuse for T {}

/// Full POSIX-compatible filesystem.
///
/// Extends [`FsFuse`] with handle-based I/O, file locking, and extended attributes.
/// This is the most complete filesystem interface, suitable for implementing
/// fully POSIX-compliant virtual filesystems.
///
/// # When to Use
///
/// Use `FsPosix` when you need:
/// - Handle-based file operations ([`FsHandles`])
/// - File locking for concurrent access ([`FsLock`])
/// - Extended attributes (xattrs) ([`FsXattr`])
///
/// # Handle-Based I/O
///
/// Unlike [`Fs`] which uses path-based operations, `FsPosix` supports opening
/// files as handles for more efficient repeated I/O:
///
/// ```rust
/// use anyfs_backend::{FsPosix, OpenFlags, LockType, FsError};
/// use std::path::Path;
///
/// // Generic function that works with any FsPosix implementation
/// fn atomic_update<B: FsPosix>(fs: &B, path: &Path, data: &[u8]) -> Result<(), FsError> {
///     // Open file
///     let handle = fs.open(path, OpenFlags::WRITE)?;
///     
///     // Acquire exclusive lock
///     fs.lock(handle, LockType::Exclusive)?;
///     
///     // Write data at offset 0
///     fs.write_at(handle, data, 0)?;
///     
///     // Release lock and close
///     fs.unlock(handle)?;
///     fs.close(handle)?;
///     
///     Ok(())
/// }
/// ```
///
/// # Extended Attributes
///
/// Store arbitrary metadata on files:
///
/// ```rust
/// use anyfs_backend::{FsPosix, FsError};
/// use std::path::Path;
///
/// // Generic function that works with any FsPosix implementation
/// fn tag_file<B: FsPosix>(fs: &B, path: &Path, tag: &str) -> Result<(), FsError> {
///     fs.set_xattr(path, "user.tag", tag.as_bytes())?;
///     
///     // Later, retrieve the tag
///     let value = fs.get_xattr(path, "user.tag")?;
///     let tag = String::from_utf8_lossy(&value);
///     println!("Tag: {}", tag);
///     
///     Ok(())
/// }
/// ```
///
/// # Blanket Implementation
///
/// Automatically implemented for any type implementing `FsFuse + FsHandles + FsLock + FsXattr`.
///
/// # Additional Methods
///
/// From [`FsHandles`]:
/// - `open`, `close` — Handle lifecycle
/// - `read_at`, `write_at` — Positioned I/O
///
/// From [`FsLock`]:
/// - `lock`, `try_lock`, `unlock` — File locking
///
/// From [`FsXattr`]:
/// - `get_xattr`, `set_xattr`, `remove_xattr`, `list_xattr` — Extended attributes
pub trait FsPosix: FsFuse + FsHandles + FsLock + FsXattr {}

// Blanket implementation
impl<T: FsFuse + FsHandles + FsLock + FsXattr> FsPosix for T {}
