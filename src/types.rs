//! Core types for the AnyFS filesystem abstraction.

use std::path::PathBuf;
use std::time::SystemTime;

/// The root directory always has inode 1 (FUSE convention).
pub const ROOT_INODE: u64 = 1;

/// Type of a filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileType {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
}

/// Metadata for a filesystem entry.
///
/// Contains all common metadata fields that can be retrieved about a file,
/// directory, or symlink.
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

/// A directory entry returned from `read_dir`.
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

/// Unix-style permissions stored as a mode bitmask.
///
/// Uses the standard Unix permission bits (rwxrwxrwx).
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

/// Filesystem statistics (like `statvfs`).
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

/// Opaque file handle for POSIX-style operations.
///
/// The internal value is backend-defined (could be an inode, index, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Handle(pub u64);

/// Flags for opening a file.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LockType {
    /// Shared lock - multiple readers allowed.
    Shared,
    /// Exclusive lock - single writer only.
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

    #[test]
    fn types_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FileType>();
        assert_send_sync::<Metadata>();
        assert_send_sync::<DirEntry>();
        assert_send_sync::<Permissions>();
        assert_send_sync::<StatFs>();
        assert_send_sync::<Handle>();
        assert_send_sync::<OpenFlags>();
        assert_send_sync::<LockType>();
    }
}
