//! # Core Types
//!
//! Fundamental types used throughout the AnyFS ecosystem.
//!
//! ## Quick Reference
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`FileType`] | Enum: File, Directory, or Symlink |
//! | [`Metadata`] | File/directory info: size, type, times, permissions |
//! | [`DirEntry`] | Single entry from a directory listing |
//! | [`Permissions`] | Unix-style permission bits (rwxrwxrwx) |
//! | [`StatFs`] | Filesystem-level statistics (total/used/available space) |
//! | [`Handle`] | Opaque file handle for POSIX-style operations |
//! | [`OpenFlags`] | Flags for opening files (read/write/create/truncate) |
//! | [`LockType`] | Shared or exclusive file lock |
//! | [`ROOT_INODE`] | Constant: root directory inode (always 1) |
//!
//! ## Serde Support
//!
//! All types support serialization when the `serde` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! anyfs-backend = { version = "0.1", features = ["serde"] }
//! ```

use std::path::PathBuf;
use std::time::SystemTime;

/// The root directory inode number (FUSE convention).
///
/// In FUSE and most Unix filesystems, inode 1 is reserved for the root directory.
/// This constant ensures consistent behavior across all AnyFS backends.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::ROOT_INODE;
///
/// assert_eq!(ROOT_INODE, 1);
/// ```
pub const ROOT_INODE: u64 = 1;

/// The type of a filesystem entry.
///
/// Every path in a filesystem is one of these three types.
///
/// # Variants
///
/// - [`File`](FileType::File) — Regular file containing data
/// - [`Directory`](FileType::Directory) — Container for other entries
/// - [`Symlink`](FileType::Symlink) — Symbolic link pointing to another path
///
/// # Example
///
/// ```rust
/// use anyfs_backend::FileType;
///
/// let ft = FileType::File;
/// assert_eq!(ft, FileType::File);
/// assert_ne!(ft, FileType::Directory);
/// ```
///
/// # Usage with Metadata
///
/// ```rust
/// use anyfs_backend::{Metadata, FileType};
///
/// let meta = Metadata::default();
/// match meta.file_type {
///     FileType::File => println!("It's a file"),
///     FileType::Directory => println!("It's a directory"),
///     FileType::Symlink => println!("It's a symlink"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileType {
    /// Regular file containing data.
    File,
    /// Directory containing other entries.
    Directory,
    /// Symbolic link pointing to another path.
    Symlink,
}

/// Complete metadata for a filesystem entry.
///
/// Contains all common metadata fields for files, directories, and symlinks.
/// Returned by [`FsRead::metadata`](crate::FsRead::metadata) and
/// [`FsInode::metadata_by_inode`](crate::FsInode::metadata_by_inode).
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `file_type` | [`FileType`] | File, Directory, or Symlink |
/// | `size` | `u64` | Size in bytes (0 for directories) |
/// | `permissions` | [`Permissions`] | Unix permission bits |
/// | `created` | `SystemTime` | Creation timestamp |
/// | `modified` | `SystemTime` | Last modification timestamp |
/// | `accessed` | `SystemTime` | Last access timestamp |
/// | `inode` | `u64` | Unique identifier within filesystem |
/// | `nlink` | `u64` | Number of hard links |
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{Metadata, FileType, Permissions};
/// use std::time::SystemTime;
///
/// let meta = Metadata {
///     file_type: FileType::File,
///     size: 1024,
///     permissions: Permissions::from_mode(0o644),
///     created: SystemTime::now(),
///     modified: SystemTime::now(),
///     accessed: SystemTime::now(),
///     inode: 42,
///     nlink: 1,
/// };
///
/// assert!(meta.is_file());
/// assert_eq!(meta.size, 1024);
/// ```
///
/// # Default Value
///
/// The default creates a zero-sized file with standard permissions (0o644):
///
/// ```rust
/// use anyfs_backend::{Metadata, FileType};
///
/// let meta = Metadata::default();
/// assert!(meta.is_file());
/// assert_eq!(meta.size, 0);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Metadata {
    /// Type of the entry (file, directory, symlink).
    pub file_type: FileType,
    /// Size in bytes.
    pub size: u64,
    /// Permissions.
    pub permissions: Permissions,
    /// Creation time.
    #[cfg_attr(feature = "serde", serde(with = "system_time_serde"))]
    pub created: SystemTime,
    /// Last modification time.
    #[cfg_attr(feature = "serde", serde(with = "system_time_serde"))]
    pub modified: SystemTime,
    /// Last access time.
    #[cfg_attr(feature = "serde", serde(with = "system_time_serde"))]
    pub accessed: SystemTime,
    /// Inode number (unique identifier within the filesystem).
    pub inode: u64,
    /// Number of hard links.
    pub nlink: u64,
}

impl Metadata {
    /// Returns `true` if this is a regular file.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::File
    }

    /// Returns `true` if this is a directory.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Directory
    }

    /// Returns `true` if this is a symbolic link.
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.file_type == FileType::Symlink
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            file_type: FileType::File,
            size: 0,
            permissions: Permissions::default_file(),
            created: SystemTime::UNIX_EPOCH,
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            inode: 0,
            nlink: 1,
        }
    }
}

/// A single entry from a directory listing.
///
/// Returned by [`FsDir::read_dir`](crate::FsDir::read_dir) via [`ReadDirIter`](crate::ReadDirIter).
/// Contains basic information about each item in a directory.
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `name` | `String` | Filename only (not full path) |
/// | `path` | `PathBuf` | Full absolute path |
/// | `file_type` | [`FileType`] | File, Directory, or Symlink |
/// | `size` | `u64` | Size in bytes |
/// | `inode` | `u64` | Inode number |
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{DirEntry, FileType};
/// use std::path::PathBuf;
///
/// let entry = DirEntry {
///     name: "readme.md".to_string(),
///     path: PathBuf::from("/docs/readme.md"),
///     file_type: FileType::File,
///     size: 2048,
///     inode: 123,
/// };
///
/// assert_eq!(entry.name, "readme.md");
/// assert_eq!(entry.file_type, FileType::File);
/// ```
///
/// # Usage with read_dir
///
/// ```rust
/// use anyfs_backend::Fs;
/// use std::path::Path;
///
/// // Generic function that works with any Fs implementation
/// fn list_files<B: Fs>(fs: &B) -> Result<(), anyfs_backend::FsError> {
///     for entry in fs.read_dir(Path::new("/"))? {
///         let entry = entry?;
///         println!("{} ({:?}, {} bytes)", entry.name, entry.file_type, entry.size);
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirEntry {
    /// Name of the entry (filename only, not full path).
    pub name: String,
    /// Full path to the entry.
    pub path: PathBuf,
    /// Type of the entry.
    pub file_type: FileType,
    /// Size in bytes.
    pub size: u64,
    /// Inode number.
    pub inode: u64,
}

/// Unix-style permission bits.
///
/// Stores permissions as a standard Unix mode bitmask (rwxrwxrwx format).
/// The lower 12 bits represent: owner (rwx), group (rwx), other (rwx), plus
/// setuid/setgid/sticky bits.
///
/// # Permission Bits
///
/// ```text
/// Mode: 0o7777 (octal)
/// ┌─────┬─────┬─────┬────────────────────┐
/// │ Special │ Owner │ Group │ Other        │
/// │ (sst)   │ (rwx) │ (rwx) │ (rwx)        │
/// └─────┴─────┴─────┴────────────────────┘
/// ```
///
/// | Bit | Meaning |
/// |-----|---------|
/// | `r` (4) | Read permission |
/// | `w` (2) | Write permission |
/// | `x` (1) | Execute/search permission |
///
/// # Common Permission Values
///
/// | Mode | Meaning |
/// |------|---------|
/// | `0o644` | Owner read/write, others read (typical file) |
/// | `0o755` | Owner all, others read/execute (typical directory) |
/// | `0o600` | Owner read/write only (private file) |
/// | `0o444` | Everyone read only |
///
/// # Example
///
/// ```rust
/// use anyfs_backend::Permissions;
///
/// // Create from octal mode
/// let perm = Permissions::from_mode(0o755);
/// assert_eq!(perm.mode(), 0o755);
/// assert!(!perm.readonly());
///
/// // Read-only permissions
/// let readonly = Permissions::from_mode(0o444);
/// assert!(readonly.readonly());
///
/// // Default permissions
/// assert_eq!(Permissions::default_file().mode(), 0o644);  // rw-r--r--
/// assert_eq!(Permissions::default_dir().mode(), 0o755);   // rwxr-xr-x
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Permissions(u32);

impl Permissions {
    /// Create permissions from a Unix mode (e.g., 0o755).
    #[inline]
    pub const fn from_mode(mode: u32) -> Self {
        Self(mode & 0o7777)
    }

    /// Get the raw mode value.
    #[inline]
    pub const fn mode(&self) -> u32 {
        self.0
    }

    /// Returns `true` if these permissions deny writing.
    #[inline]
    pub const fn readonly(&self) -> bool {
        // Check if no write bits are set (user, group, or other)
        (self.0 & 0o222) == 0
    }

    /// Default permissions for a new file (0o644 = rw-r--r--).
    #[inline]
    pub const fn default_file() -> Self {
        Self(0o644)
    }

    /// Default permissions for a new directory (0o755 = rwxr-xr-x).
    #[inline]
    pub const fn default_dir() -> Self {
        Self(0o755)
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::default_file()
    }
}

/// Filesystem-level statistics.
///
/// Returned by [`FsStats::statfs`](crate::FsStats::statfs). Contains information
/// about the filesystem's capacity, usage, and limits — similar to the POSIX
/// `statvfs` system call.
///
/// # Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `total_bytes` | `u64` | Total capacity (0 = unlimited) |
/// | `used_bytes` | `u64` | Currently used space |
/// | `available_bytes` | `u64` | Space available for new data |
/// | `total_inodes` | `u64` | Maximum files/directories (0 = unlimited) |
/// | `used_inodes` | `u64` | Currently allocated inodes |
/// | `available_inodes` | `u64` | Inodes available for new entries |
/// | `block_size` | `u64` | Filesystem block size in bytes |
/// | `max_name_len` | `u64` | Maximum filename length |
///
/// # Example
///
/// ```rust
/// use anyfs_backend::StatFs;
///
/// let stats = StatFs {
///     total_bytes: 1_000_000_000,        // 1 GB
///     used_bytes: 250_000_000,           // 250 MB used
///     available_bytes: 750_000_000,      // 750 MB free
///     total_inodes: 100_000,
///     used_inodes: 1_234,
///     available_inodes: 98_766,
///     block_size: 4096,
///     max_name_len: 255,
/// };
///
/// let usage_percent = (stats.used_bytes as f64 / stats.total_bytes as f64) * 100.0;
/// println!("Disk usage: {:.1}%", usage_percent);  // "Disk usage: 25.0%"
/// ```
///
/// # Unlimited Filesystems
///
/// For backends without fixed limits (e.g., cloud storage), set capacity fields to 0:
///
/// ```rust
/// use anyfs_backend::StatFs;
///
/// let unlimited = StatFs {
///     total_bytes: 0,      // No limit
///     total_inodes: 0,     // No limit
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatFs {
    /// Total size in bytes (0 = unlimited).
    pub total_bytes: u64,
    /// Currently used bytes.
    pub used_bytes: u64,
    /// Available bytes for use.
    pub available_bytes: u64,
    /// Total number of inodes (0 = unlimited).
    pub total_inodes: u64,
    /// Number of used inodes.
    pub used_inodes: u64,
    /// Number of available inodes.
    pub available_inodes: u64,
    /// Block size in bytes.
    pub block_size: u64,
    /// Maximum filename length.
    pub max_name_len: u64,
}

/// Opaque file handle for POSIX-style I/O operations.
///
/// Represents an open file descriptor. Used with [`FsHandles`](crate::FsHandles)
/// for handle-based read/write operations, and with [`FsLock`](crate::FsLock)
/// for file locking.
///
/// # Lifecycle
///
/// 1. **Open**: Call [`FsHandles::open`](crate::FsHandles::open) to get a handle
/// 2. **Use**: Call `read_at`, `write_at`, `lock`, etc.
/// 3. **Close**: Call [`FsHandles::close`](crate::FsHandles::close) to release
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsHandles, OpenFlags, Handle};
/// use std::path::Path;
///
/// // Generic function that works with any FsHandles implementation
/// fn write_with_handle<B: FsHandles>(fs: &B) -> Result<(), anyfs_backend::FsError> {
///     // Open file for writing
///     let handle: Handle = fs.open(Path::new("/data.bin"), OpenFlags::WRITE)?;
///     
///     // Write at specific offset
///     fs.write_at(handle, b"Hello", 0)?;
///     fs.write_at(handle, b"World", 5)?;
///     
///     // Always close the handle
///     fs.close(handle)?;
///     Ok(())
/// }
/// ```
///
/// # Internal Value
///
/// The `u64` value is backend-defined. It could be an inode number, array index,
/// or any other unique identifier. Treat it as opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Handle(pub u64);

/// Flags for opening a file.
///
/// Controls how a file is opened: read/write mode, creation behavior, and
/// truncation. Used with [`FsHandles::open`](crate::FsHandles::open).
///
/// # Predefined Constants
///
/// | Constant | Behavior |
/// |----------|----------|
/// | [`OpenFlags::READ`] | Read-only access |
/// | [`OpenFlags::WRITE`] | Write with create and truncate |
/// | [`OpenFlags::READ_WRITE`] | Read and write, file must exist |
/// | [`OpenFlags::APPEND`] | Append mode (writes go to end) |
///
/// # Fields
///
/// | Field | Effect |
/// |-------|--------|
/// | `read` | Enable reading from file |
/// | `write` | Enable writing to file |
/// | `create` | Create file if it doesn't exist |
/// | `truncate` | Truncate file to zero length on open |
/// | `append` | Writes always go to end of file |
///
/// # Example
///
/// ```rust
/// use anyfs_backend::OpenFlags;
///
/// // Use predefined constants
/// let read_only = OpenFlags::READ;
/// assert!(read_only.read);
/// assert!(!read_only.write);
///
/// // Custom flags
/// let custom = OpenFlags {
///     read: true,
///     write: true,
///     create: true,
///     truncate: false,
///     append: false,
/// };
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpenFlags {
    /// Open for reading.
    pub read: bool,
    /// Open for writing.
    pub write: bool,
    /// Create file if it doesn't exist.
    pub create: bool,
    /// Truncate file to zero length.
    pub truncate: bool,
    /// Append to end of file.
    pub append: bool,
}

impl OpenFlags {
    /// Read-only access.
    pub const READ: Self = Self {
        read: true,
        write: false,
        create: false,
        truncate: false,
        append: false,
    };

    /// Write access with create and truncate.
    pub const WRITE: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: true,
        append: false,
    };

    /// Read and write access.
    pub const READ_WRITE: Self = Self {
        read: true,
        write: true,
        create: false,
        truncate: false,
        append: false,
    };

    /// Append mode - writes go to end of file.
    pub const APPEND: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: false,
        append: true,
    };
}

/// Type of file lock.
///
/// Used with [`FsLock::lock`](crate::FsLock::lock) to request either shared
/// or exclusive access to a file.
///
/// # Variants
///
/// | Variant | Behavior |
/// |---------|----------|
/// | [`Shared`](LockType::Shared) | Multiple readers allowed simultaneously |
/// | [`Exclusive`](LockType::Exclusive) | Single writer, no other access |
///
/// # Lock Compatibility
///
/// | Held Lock | Shared Request | Exclusive Request |
/// |-----------|----------------|-------------------|
/// | None | ✓ Granted | ✓ Granted |
/// | Shared | ✓ Granted | ✗ Blocked |
/// | Exclusive | ✗ Blocked | ✗ Blocked |
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsLock, FsHandles, OpenFlags, LockType};
/// use std::path::Path;
///
/// // Generic function that works with any FsHandles + FsLock implementation
/// fn safe_read<B: FsHandles + FsLock>(fs: &B) -> Result<(), anyfs_backend::FsError> {
///     let handle = fs.open(Path::new("/data.txt"), OpenFlags::READ)?;
///     
///     // Shared lock allows concurrent readers
///     fs.lock(handle, LockType::Shared)?;
///     
///     // ... read data ...
///     
///     fs.unlock(handle)?;
///     fs.close(handle)?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LockType {
    /// Shared lock — multiple readers allowed simultaneously.
    ///
    /// Use when reading data that shouldn't change during the read.
    /// Multiple processes can hold shared locks on the same file.
    Shared,

    /// Exclusive lock — single writer, blocks all other access.
    ///
    /// Use when modifying data. Only one process can hold an exclusive
    /// lock, and it blocks all shared lock requests.
    Exclusive,
}

/// Serde support for SystemTime (when serde feature is enabled).
#[cfg(feature = "serde")]
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        (duration.as_secs(), duration.subsec_nanos()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (secs, nanos): (u64, u32) = Deserialize::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::new(secs, nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_type_equality() {
        assert_eq!(FileType::File, FileType::File);
        assert_ne!(FileType::File, FileType::Directory);
    }

    #[test]
    fn metadata_is_file() {
        let m = Metadata {
            file_type: FileType::File,
            ..Default::default()
        };
        assert!(m.is_file());
        assert!(!m.is_dir());
        assert!(!m.is_symlink());
    }

    #[test]
    fn metadata_is_dir() {
        let m = Metadata {
            file_type: FileType::Directory,
            ..Default::default()
        };
        assert!(!m.is_file());
        assert!(m.is_dir());
        assert!(!m.is_symlink());
    }

    #[test]
    fn metadata_is_symlink() {
        let m = Metadata {
            file_type: FileType::Symlink,
            ..Default::default()
        };
        assert!(!m.is_file());
        assert!(!m.is_dir());
        assert!(m.is_symlink());
    }

    #[test]
    fn permissions_from_mode() {
        let p = Permissions::from_mode(0o755);
        assert_eq!(p.mode(), 0o755);
    }

    #[test]
    fn permissions_from_mode_masks_extra_bits() {
        let p = Permissions::from_mode(0o100755);
        assert_eq!(p.mode(), 0o755);
    }

    #[test]
    fn permissions_readonly() {
        let readonly = Permissions::from_mode(0o444);
        assert!(readonly.readonly());

        let writable = Permissions::from_mode(0o644);
        assert!(!writable.readonly());
    }

    #[test]
    fn permissions_defaults() {
        assert_eq!(Permissions::default_file().mode(), 0o644);
        assert_eq!(Permissions::default_dir().mode(), 0o755);
    }

    #[test]
    fn open_flags_constants() {
        assert!(OpenFlags::READ.read);
        assert!(!OpenFlags::READ.write);
        assert!(!OpenFlags::READ.create);

        assert!(!OpenFlags::WRITE.read);
        assert!(OpenFlags::WRITE.write);
        assert!(OpenFlags::WRITE.create);
        assert!(OpenFlags::WRITE.truncate);

        assert!(OpenFlags::READ_WRITE.read);
        assert!(OpenFlags::READ_WRITE.write);
        assert!(!OpenFlags::READ_WRITE.create);

        assert!(OpenFlags::APPEND.write);
        assert!(OpenFlags::APPEND.create);
        assert!(OpenFlags::APPEND.append);
        assert!(!OpenFlags::APPEND.truncate);
    }

    #[test]
    fn lock_type_equality() {
        assert_eq!(LockType::Shared, LockType::Shared);
        assert_eq!(LockType::Exclusive, LockType::Exclusive);
        assert_ne!(LockType::Shared, LockType::Exclusive);
    }

    #[test]
    fn root_inode_is_one() {
        assert_eq!(ROOT_INODE, 1);
    }

    #[test]
    fn handle_equality() {
        assert_eq!(Handle(42), Handle(42));
        assert_ne!(Handle(1), Handle(2));
    }
}
