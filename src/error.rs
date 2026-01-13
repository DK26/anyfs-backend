//! # Error Types
//!
//! Comprehensive error handling for AnyFS filesystem operations.
//!
//! ## Overview
//!
//! All AnyFS operations return `Result<T, FsError>`. The [`FsError`] enum provides
//! detailed, contextual error variants that include:
//!
//! - **Path information** — Which file/directory caused the error
//! - **Operation context** — What operation was attempted
//! - **Specific details** — Quota limits, invalid data descriptions, etc.
//!
//! ## Error Categories
//!
//! | Category | Variants | Description |
//! |----------|----------|-------------|
//! | Path/File | `NotFound`, `AlreadyExists`, `NotAFile`, `NotADirectory` | Path existence and type errors |
//! | Permission | `PermissionDenied`, `AccessDenied`, `ReadOnly` | Access control errors |
//! | Resource | `QuotaExceeded`, `FileSizeExceeded`, `RateLimitExceeded` | Limit violations |
//! | Data | `InvalidData`, `CorruptedData`, `IntegrityError` | Content problems |
//! | Operation | `NotSupported`, `Conflict`, `Backend` | Backend/operation failures |
//!
//! ## Quick Example
//!
//! ```rust
//! use anyfs_backend::FsError;
//! use std::path::PathBuf;
//!
//! // Errors include the path that caused the problem
//! let err = FsError::NotFound { path: PathBuf::from("/missing.txt") };
//! assert!(err.to_string().contains("/missing.txt"));
//!
//! // Permission errors include the operation
//! let err = FsError::PermissionDenied {
//!     path: PathBuf::from("/secret"),
//!     operation: "read",
//! };
//! assert!(err.to_string().contains("read"));
//! ```
//!
//! ## Conversion from std::io::Error
//!
//! [`FsError`] implements `From<std::io::Error>` for easy interoperability:
//!
//! ```rust
//! use anyfs_backend::FsError;
//! use std::io::{Error, ErrorKind};
//!
//! let io_err = Error::new(ErrorKind::NotFound, "file not found");
//! let fs_err: FsError = io_err.into();
//! assert!(matches!(fs_err, FsError::NotFound { .. }));
//! ```

use std::path::PathBuf;

/// Comprehensive filesystem error type.
///
/// All AnyFS operations return `Result<T, FsError>`. Each variant includes
/// relevant context (paths, operations, limits) to make debugging easier.
///
/// # Non-Exhaustive
///
/// This enum is marked `#[non_exhaustive]`, meaning new variants may be added
/// in future versions without breaking changes. Always include a wildcard arm
/// when pattern matching:
///
/// ```rust
/// use anyfs_backend::FsError;
/// use std::path::PathBuf;
///
/// fn handle_error(err: FsError) {
///     match err {
///         FsError::NotFound { path } => println!("Not found: {}", path.display()),
///         FsError::PermissionDenied { path, operation } => {
///             println!("Permission denied for {} on {}", operation, path.display())
///         }
///         other => println!("Other error: {}", other),
///     }
/// }
/// ```
///
/// # Display Format
///
/// All variants implement `Display` with human-readable messages:
///
/// ```rust
/// use anyfs_backend::FsError;
/// use std::path::PathBuf;
///
/// let err = FsError::QuotaExceeded { limit: 100, requested: 50, usage: 80 };
/// let msg = err.to_string();
/// assert!(msg.contains("100") && msg.contains("50") && msg.contains("80"));
/// ```
///
/// # Error Source Chain
///
/// The [`Io`](FsError::Io) variant wraps `std::io::Error` with the `#[source]`
/// attribute, enabling error chain traversal via `std::error::Error::source()`.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    // Path/File Errors
    /// Path does not exist.
    #[error("not found: {path}")]
    NotFound {
        /// The path that was not found.
        path: PathBuf,
    },

    /// A threat was detected (e.g., path traversal, malicious content).
    #[error("threat detected: {reason} in {path}")]
    ThreatDetected {
        /// The path where the threat was detected.
        path: PathBuf,
        /// Description of the threat.
        reason: String,
    },

    /// Path already exists when it shouldn't.
    #[error("{operation}: already exists: {path}")]
    AlreadyExists {
        /// The path that already exists.
        path: PathBuf,
        /// The operation that failed.
        operation: &'static str,
    },

    /// Expected a file but found something else.
    #[error("not a file: {path}")]
    NotAFile {
        /// The path that is not a file.
        path: PathBuf,
    },

    /// Expected a directory but found something else.
    #[error("not a directory: {path}")]
    NotADirectory {
        /// The path that is not a directory.
        path: PathBuf,
    },

    /// Directory is not empty when it should be.
    #[error("directory not empty: {path}")]
    DirectoryNotEmpty {
        /// The path to the non-empty directory.
        path: PathBuf,
    },

    /// Inode does not exist.
    #[error("inode not found: {inode}")]
    InodeNotFound {
        /// The inode number that was not found.
        inode: u64,
    },

    /// File handle is invalid or closed.
    #[error("invalid handle: {}", handle.0)]
    InvalidHandle {
        /// The invalid handle.
        handle: crate::Handle,
    },

    /// Extended attribute not found.
    #[error("xattr not found: {name} on {path}")]
    XattrNotFound {
        /// The path where the xattr was not found.
        path: PathBuf,
        /// The attribute name that was not found.
        name: String,
    },

    // Permission/Access Errors
    /// Permission denied for operation.
    #[error("{operation}: permission denied: {path}")]
    PermissionDenied {
        /// The path where permission was denied.
        path: PathBuf,
        /// The operation that was denied.
        operation: &'static str,
    },

    /// Access denied with reason.
    #[error("access denied: {path} ({reason})")]
    AccessDenied {
        /// The path where access was denied.
        path: PathBuf,
        /// The reason for denial.
        reason: String,
    },

    /// Filesystem is read-only.
    #[error("read-only filesystem: {operation}")]
    ReadOnly {
        /// The operation that was attempted.
        operation: &'static str,
    },

    /// Feature is not enabled.
    #[error("{operation}: feature not enabled: {feature}")]
    FeatureNotEnabled {
        /// The feature that is not enabled.
        feature: &'static str,
        /// The operation that requires the feature.
        operation: &'static str,
    },

    // Resource Limit Errors
    /// Quota exceeded.
    #[error("quota exceeded: limit {limit}, requested {requested}, usage {usage}")]
    QuotaExceeded {
        /// The quota limit.
        limit: u64,
        /// The amount requested.
        requested: u64,
        /// The current usage.
        usage: u64,
    },

    /// File size limit exceeded.
    #[error("file size exceeded: {path} ({size} > {limit})")]
    FileSizeExceeded {
        /// The path to the file.
        path: PathBuf,
        /// The actual size.
        size: u64,
        /// The size limit.
        limit: u64,
    },

    /// Rate limit exceeded.
    #[error("rate limit exceeded: {limit}/s (window: {window_secs}s)")]
    RateLimitExceeded {
        /// The rate limit.
        limit: u32,
        /// The time window in seconds.
        window_secs: u64,
    },

    // Data Errors
    /// Invalid data encountered.
    #[error("invalid data: {path} ({details})")]
    InvalidData {
        /// The path with invalid data.
        path: PathBuf,
        /// Details about the invalid data.
        details: String,
    },

    /// Corrupted data detected.
    #[error("corrupted data: {path} ({details})")]
    CorruptedData {
        /// The path with corrupted data.
        path: PathBuf,
        /// Details about the corruption.
        details: String,
    },

    /// Data integrity check failed.
    #[error("integrity error: {path}")]
    IntegrityError {
        /// The path that failed integrity check.
        path: PathBuf,
    },

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    // Backend/Operation Errors
    /// Operation is not supported.
    #[error("operation not supported: {operation}")]
    NotSupported {
        /// The unsupported operation.
        operation: &'static str,
    },

    /// Invalid password provided.
    #[error("invalid password")]
    InvalidPassword,

    /// Conflict detected (e.g., concurrent modification).
    #[error("conflict: {path}")]
    Conflict {
        /// The path with a conflict.
        path: PathBuf,
    },

    /// Generic backend error.
    #[error("backend error: {0}")]
    Backend(String),

    /// I/O error with context.
    #[error("{operation} failed for {path}: {source}")]
    Io {
        /// The operation that failed.
        operation: &'static str,
        /// The path involved in the operation.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl From<std::io::Error> for FsError {
    fn from(error: std::io::Error) -> Self {
        // Convert common io::ErrorKind to more specific FsError variants when possible
        match error.kind() {
            std::io::ErrorKind::NotFound => FsError::NotFound {
                path: PathBuf::new(),
            },
            std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: PathBuf::new(),
                operation: "io",
            },
            std::io::ErrorKind::AlreadyExists => FsError::AlreadyExists {
                path: PathBuf::new(),
                operation: "io",
            },
            _ => FsError::Io {
                operation: "io",
                path: PathBuf::new(),
                source: error,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_error_not_found_display() {
        let err = FsError::NotFound {
            path: PathBuf::from("/missing"),
        };
        assert_eq!(err.to_string(), "not found: /missing");
    }

    #[test]
    fn fs_error_already_exists_display() {
        let err = FsError::AlreadyExists {
            path: PathBuf::from("/exists"),
            operation: "create",
        };
        assert_eq!(err.to_string(), "create: already exists: /exists");
    }

    #[test]
    fn fs_error_quota_exceeded_display() {
        let err = FsError::QuotaExceeded {
            limit: 100,
            requested: 50,
            usage: 80,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
        assert!(err.to_string().contains("80"));
    }

    #[test]
    fn fs_error_from_io_not_found() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let fs_err = FsError::from(io_err);
        assert!(matches!(fs_err, FsError::NotFound { .. }));
    }

    #[test]
    fn fs_error_from_io_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test");
        let fs_err = FsError::from(io_err);
        assert!(matches!(fs_err, FsError::PermissionDenied { .. }));
    }

    #[test]
    fn fs_error_from_io_already_exists() {
        let io_err = std::io::Error::new(std::io::ErrorKind::AlreadyExists, "test");
        let fs_err = FsError::from(io_err);
        assert!(matches!(fs_err, FsError::AlreadyExists { .. }));
    }

    #[test]
    fn fs_error_from_io_other() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let fs_err = FsError::from(io_err);
        assert!(matches!(fs_err, FsError::Io { .. }));
    }
}
