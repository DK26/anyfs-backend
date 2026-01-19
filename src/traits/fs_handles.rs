//! Handle-based file operations for POSIX compatibility.
//!
//! This module provides the [`FsHandles`] trait which enables file operations
//! using opaque handles instead of paths. This is essential for POSIX-compliant
//! filesystems where files can be modified after being opened.
//!
//! # Overview
//!
//! Handle-based operations are more efficient for repeated access to the same
//! file and are required for full POSIX compatibility. The workflow is:
//!
//! 1. `open()` - Open a file and get a handle
//! 2. `read_at()` / `write_at()` - Perform operations using the handle
//! 3. `close()` - Release the handle
//!
//! # Example
//!
//! ```rust
//! use anyfs_backend::{FsHandles, OpenFlags, FsError};
//! use std::path::Path;
//!
//! // Generic function that works with any FsHandles implementation
//! fn copy_via_handles<B: FsHandles>(
//!     backend: &B,
//!     src: &Path,
//!     dst: &Path,
//! ) -> Result<(), FsError> {
//!     let src_handle = backend.open(src, OpenFlags::READ)?;
//!     let dst_handle = backend.open(dst, OpenFlags::WRITE)?;
//!
//!     let mut buf = [0u8; 4096];
//!     let mut offset = 0;
//!     loop {
//!         let n = backend.read_at(src_handle, &mut buf, offset)?;
//!         if n == 0 {
//!             break;
//!         }
//!         backend.write_at(dst_handle, &buf[..n], offset)?;
//!         offset += n as u64;
//!     }
//!
//!     backend.close(src_handle)?;
//!     backend.close(dst_handle)?;
//!     Ok(())
//! }
//! ```
//!
//! # Thread Safety
//!
//! Like all AnyFS traits, `FsHandles` requires `Send + Sync`. Implementations
//! must use interior mutability and handle concurrent access properly.

use std::path::Path;

use crate::{FsError, Handle, OpenFlags};

/// Handle-based file operations for POSIX compatibility.
///
/// This trait provides low-level file operations using opaque [`Handle`]s.
/// These operations are essential for FUSE mounts and POSIX-compliant
/// applications that need to open a file once and perform multiple
/// operations on it.
///
/// # Handle Lifecycle
///
/// 1. Obtain a handle via [`open`](FsHandles::open)
/// 2. Perform operations with [`read_at`](FsHandles::read_at) and
///    [`write_at`](FsHandles::write_at)
/// 3. Release the handle with [`close`](FsHandles::close)
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsHandles, OpenFlags, FsError, Handle};
/// use std::path::Path;
///
/// // Generic function that works with any FsHandles implementation
/// fn read_header<B: FsHandles>(backend: &B, path: &Path) -> Result<[u8; 16], FsError> {
///     let handle = backend.open(path, OpenFlags::READ)?;
///     let mut header = [0u8; 16];
///     backend.read_at(handle, &mut header, 0)?;
///     backend.close(handle)?;
///     Ok(header)
/// }
/// ```
pub trait FsHandles: Send + Sync {
    /// Open a file and return a handle.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path to open
    /// * `flags` - Open flags specifying read/write mode and creation behavior
    ///
    /// # Returns
    ///
    /// An opaque [`Handle`] that can be used for subsequent operations.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the file doesn't exist and `create` is false
    /// - [`FsError::AlreadyExists`] if the file exists and exclusive creation is requested
    /// - [`FsError::NotAFile`] if the path is a directory
    /// - [`FsError::PermissionDenied`] if access is denied
    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Handle, FsError>;

    /// Read data from a file at a specific offset.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle obtained from [`open`](FsHandles::open)
    /// * `buf` - Buffer to read data into
    /// * `offset` - Byte offset to start reading from
    ///
    /// # Returns
    ///
    /// Number of bytes read. Returns 0 at end of file.
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidHandle`] if the handle is invalid or closed
    /// - [`FsError::PermissionDenied`] if the handle wasn't opened for reading
    fn read_at(&self, handle: Handle, buf: &mut [u8], offset: u64) -> Result<usize, FsError>;

    /// Write data to a file at a specific offset.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle obtained from [`open`](FsHandles::open)
    /// * `data` - Data to write
    /// * `offset` - Byte offset to start writing at
    ///
    /// # Returns
    ///
    /// Number of bytes written.
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidHandle`] if the handle is invalid or closed
    /// - [`FsError::PermissionDenied`] if the handle wasn't opened for writing
    /// - [`FsError::QuotaExceeded`] if storage quota is exceeded
    fn write_at(&self, handle: Handle, data: &[u8], offset: u64) -> Result<usize, FsError>;

    /// Close a file handle.
    ///
    /// Releases any resources associated with the handle. After closing,
    /// the handle becomes invalid and must not be used.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle to close
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidHandle`] if the handle is already closed or invalid
    fn close(&self, handle: Handle) -> Result<(), FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::RwLock;

    /// Information about an open file handle
    struct OpenFile {
        data: Vec<u8>,
        flags: OpenFlags,
    }

    /// Mock implementation of FsHandles for testing
    struct MockHandleFs {
        // Next handle ID
        next_handle: AtomicU64,
        // Open handles -> file data and flags
        handles: RwLock<HashMap<u64, OpenFile>>,
        // File storage: path -> data
        files: RwLock<HashMap<std::path::PathBuf, Vec<u8>>>,
    }

    impl MockHandleFs {
        fn new() -> Self {
            Self {
                next_handle: AtomicU64::new(1),
                handles: RwLock::new(HashMap::new()),
                files: RwLock::new(HashMap::new()),
            }
        }

        fn create_file(&self, path: &Path, data: Vec<u8>) {
            self.files.write().unwrap().insert(path.to_path_buf(), data);
        }
    }

    impl FsHandles for MockHandleFs {
        fn open(&self, path: &Path, flags: OpenFlags) -> Result<Handle, FsError> {
            let files = self.files.read().unwrap();

            let data = if flags.create {
                // Create or truncate
                if flags.truncate {
                    Vec::new()
                } else {
                    files.get(path).cloned().unwrap_or_default()
                }
            } else {
                // Must exist
                files.get(path).cloned().ok_or_else(|| FsError::NotFound {
                    path: path.to_path_buf(),
                })?
            };

            drop(files);

            let handle_id = self.next_handle.fetch_add(1, Ordering::SeqCst);
            self.handles
                .write()
                .unwrap()
                .insert(handle_id, OpenFile { data, flags });

            Ok(Handle(handle_id))
        }

        fn read_at(&self, handle: Handle, buf: &mut [u8], offset: u64) -> Result<usize, FsError> {
            let handles = self.handles.read().unwrap();
            let open_file = handles
                .get(&handle.0)
                .ok_or(FsError::InvalidHandle { handle })?;

            if !open_file.flags.read {
                return Err(FsError::PermissionDenied {
                    path: std::path::PathBuf::new(),
                    operation: "read_at",
                });
            }

            let offset = offset as usize;
            if offset >= open_file.data.len() {
                return Ok(0);
            }

            let available = open_file.data.len() - offset;
            let to_read = buf.len().min(available);
            buf[..to_read].copy_from_slice(&open_file.data[offset..offset + to_read]);
            Ok(to_read)
        }

        fn write_at(&self, handle: Handle, data: &[u8], offset: u64) -> Result<usize, FsError> {
            let mut handles = self.handles.write().unwrap();
            let open_file = handles
                .get_mut(&handle.0)
                .ok_or(FsError::InvalidHandle { handle })?;

            if !open_file.flags.write {
                return Err(FsError::PermissionDenied {
                    path: std::path::PathBuf::new(),
                    operation: "write_at",
                });
            }

            let offset = offset as usize;
            let end = offset + data.len();

            // Extend file if necessary
            if end > open_file.data.len() {
                open_file.data.resize(end, 0);
            }

            open_file.data[offset..end].copy_from_slice(data);
            Ok(data.len())
        }

        fn close(&self, handle: Handle) -> Result<(), FsError> {
            self.handles
                .write()
                .unwrap()
                .remove(&handle.0)
                .map(|_| ())
                .ok_or(FsError::InvalidHandle { handle })
        }
    }

    #[test]
    fn open_existing_file() {
        let fs = MockHandleFs::new();
        fs.create_file(Path::new("/test.txt"), b"hello".to_vec());

        let handle = fs.open(Path::new("/test.txt"), OpenFlags::READ).unwrap();
        assert_eq!(handle.0, 1);
        fs.close(handle).unwrap();
    }

    #[test]
    fn open_nonexistent_file_fails() {
        let fs = MockHandleFs::new();
        let result = fs.open(Path::new("/missing.txt"), OpenFlags::READ);
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn open_with_create() {
        let fs = MockHandleFs::new();
        let handle = fs.open(Path::new("/new.txt"), OpenFlags::WRITE).unwrap();
        fs.close(handle).unwrap();
    }

    #[test]
    fn read_at_returns_data() {
        let fs = MockHandleFs::new();
        fs.create_file(Path::new("/test.txt"), b"hello world".to_vec());

        let handle = fs.open(Path::new("/test.txt"), OpenFlags::READ).unwrap();
        let mut buf = [0u8; 5];
        let n = fs.read_at(handle, &mut buf, 0).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");
        fs.close(handle).unwrap();
    }

    #[test]
    fn read_at_with_offset() {
        let fs = MockHandleFs::new();
        fs.create_file(Path::new("/test.txt"), b"hello world".to_vec());

        let handle = fs.open(Path::new("/test.txt"), OpenFlags::READ).unwrap();
        let mut buf = [0u8; 5];
        let n = fs.read_at(handle, &mut buf, 6).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"world");
        fs.close(handle).unwrap();
    }

    #[test]
    fn read_at_past_eof_returns_zero() {
        let fs = MockHandleFs::new();
        fs.create_file(Path::new("/test.txt"), b"hi".to_vec());

        let handle = fs.open(Path::new("/test.txt"), OpenFlags::READ).unwrap();
        let mut buf = [0u8; 5];
        let n = fs.read_at(handle, &mut buf, 100).unwrap();
        assert_eq!(n, 0);
        fs.close(handle).unwrap();
    }

    #[test]
    fn read_without_read_permission_fails() {
        let fs = MockHandleFs::new();
        fs.create_file(Path::new("/test.txt"), b"hello".to_vec());

        // Open with write-only
        let flags = OpenFlags {
            read: false,
            write: true,
            create: false,
            truncate: false,
            append: false,
        };
        let handle = fs.open(Path::new("/test.txt"), flags).unwrap();
        let mut buf = [0u8; 5];
        let result = fs.read_at(handle, &mut buf, 0);
        assert!(matches!(result, Err(FsError::PermissionDenied { .. })));
        fs.close(handle).unwrap();
    }

    #[test]
    fn write_at_appends_data() {
        let fs = MockHandleFs::new();
        let handle = fs.open(Path::new("/new.txt"), OpenFlags::WRITE).unwrap();

        let n = fs.write_at(handle, b"hello", 0).unwrap();
        assert_eq!(n, 5);

        // Read back to verify
        let handles = fs.handles.read().unwrap();
        let open_file = handles.get(&handle.0).unwrap();
        assert_eq!(&open_file.data, b"hello");
        drop(handles);

        fs.close(handle).unwrap();
    }

    #[test]
    fn write_without_write_permission_fails() {
        let fs = MockHandleFs::new();
        fs.create_file(Path::new("/test.txt"), b"hello".to_vec());

        let handle = fs.open(Path::new("/test.txt"), OpenFlags::READ).unwrap();
        let result = fs.write_at(handle, b"new data", 0);
        assert!(matches!(result, Err(FsError::PermissionDenied { .. })));
        fs.close(handle).unwrap();
    }

    #[test]
    fn close_invalid_handle_fails() {
        let fs = MockHandleFs::new();
        let result = fs.close(Handle(999));
        assert!(matches!(result, Err(FsError::InvalidHandle { .. })));
    }

    #[test]
    fn close_then_use_fails() {
        let fs = MockHandleFs::new();
        fs.create_file(Path::new("/test.txt"), b"hello".to_vec());

        let handle = fs.open(Path::new("/test.txt"), OpenFlags::READ).unwrap();
        fs.close(handle).unwrap();

        // Using closed handle should fail
        let mut buf = [0u8; 5];
        let result = fs.read_at(handle, &mut buf, 0);
        assert!(matches!(result, Err(FsError::InvalidHandle { .. })));
    }
}
