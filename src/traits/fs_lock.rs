//! File locking operations for POSIX compatibility.
//!
//! This module provides the [`FsLock`] trait which enables file locking
//! operations using handles. This is essential for POSIX-compliant
//! applications that need to coordinate concurrent file access.
//!
//! # Overview
//!
//! File locking allows processes to coordinate access to shared files:
//!
//! - **Shared locks**: Multiple readers can hold a shared lock simultaneously
//! - **Exclusive locks**: Only one writer can hold an exclusive lock
//!
//! # Example
//!
//! ```rust
//! use anyfs_backend::{FsLock, FsHandles, LockType, OpenFlags, FsError};
//! use std::path::Path;
//!
//! // Generic function that works with any FsHandles + FsLock implementation
//! fn exclusive_update<B: FsHandles + FsLock>(
//!     backend: &B,
//!     path: &Path,
//!     data: &[u8],
//! ) -> Result<(), FsError> {
//!     let handle = backend.open(path, OpenFlags::READ_WRITE)?;
//!     
//!     // Get exclusive lock
//!     backend.lock(handle, LockType::Exclusive)?;
//!     
//!     // Perform update
//!     backend.write_at(handle, data, 0)?;
//!     
//!     // Release lock
//!     backend.unlock(handle)?;
//!     backend.close(handle)?;
//!     Ok(())
//! }
//! ```
//!
//! # Thread Safety
//!
//! Like all AnyFS traits, `FsLock` requires `Send + Sync`. Implementations
//! must handle concurrent lock requests appropriately.

use crate::{FsError, Handle, LockType};

/// File locking operations for POSIX compatibility.
///
/// This trait provides POSIX-style file locking using [`Handle`]s and
/// [`LockType`]. Locks are advisory - they provide coordination between
/// cooperating processes but don't prevent other access.
///
/// # Lock Types
///
/// - [`LockType::Shared`] - Multiple readers, blocks exclusive locks
/// - [`LockType::Exclusive`] - Single writer, blocks all other locks
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{FsLock, LockType, Handle, FsError};
///
/// // Generic function that works with any FsLock implementation
/// fn with_shared_lock<B: FsLock>(
///     backend: &B,
///     handle: Handle,
/// ) -> Result<(), FsError> {
///     backend.lock(handle, LockType::Shared)?;
///     // ... read operations ...
///     backend.unlock(handle)?;
///     Ok(())
/// }
/// ```
pub trait FsLock: Send + Sync {
    /// Acquire a lock on a file handle.
    ///
    /// This is a blocking operation - it will wait until the lock can be
    /// acquired.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle to the open file
    /// * `lock` - Type of lock to acquire
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidHandle`] if the handle is invalid or closed
    /// - [`FsError::NotSupported`] if locking is not supported
    fn lock(&self, handle: Handle, lock: LockType) -> Result<(), FsError>;

    /// Try to acquire a lock without blocking.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle to the open file
    /// * `lock` - Type of lock to acquire
    ///
    /// # Returns
    ///
    /// `true` if the lock was acquired, `false` if it would block.
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidHandle`] if the handle is invalid or closed
    /// - [`FsError::NotSupported`] if locking is not supported
    fn try_lock(&self, handle: Handle, lock: LockType) -> Result<bool, FsError>;

    /// Release a lock on a file handle.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle to the open file
    ///
    /// # Errors
    ///
    /// - [`FsError::InvalidHandle`] if the handle is invalid or closed
    fn unlock(&self, handle: Handle) -> Result<(), FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// Lock state for a handle
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum LockState {
        Unlocked,
        Shared(usize), // Count of shared locks
        Exclusive,
    }

    /// Mock implementation of FsLock for testing
    struct MockLockFs {
        locks: RwLock<HashMap<u64, LockState>>,
        valid_handles: RwLock<std::collections::HashSet<u64>>,
    }

    impl MockLockFs {
        fn new() -> Self {
            Self {
                locks: RwLock::new(HashMap::new()),
                valid_handles: RwLock::new(std::collections::HashSet::new()),
            }
        }

        fn add_handle(&self, handle: u64) {
            self.valid_handles.write().unwrap().insert(handle);
        }
    }

    impl FsLock for MockLockFs {
        fn lock(&self, handle: Handle, lock_type: LockType) -> Result<(), FsError> {
            if !self.valid_handles.read().unwrap().contains(&handle.0) {
                return Err(FsError::InvalidHandle { handle });
            }

            let mut locks = self.locks.write().unwrap();
            let state = locks.entry(handle.0).or_insert(LockState::Unlocked);

            match (*state, lock_type) {
                (LockState::Unlocked, LockType::Shared) => {
                    *state = LockState::Shared(1);
                    Ok(())
                }
                (LockState::Unlocked, LockType::Exclusive) => {
                    *state = LockState::Exclusive;
                    Ok(())
                }
                (LockState::Shared(n), LockType::Shared) => {
                    *state = LockState::Shared(n + 1);
                    Ok(())
                }
                (LockState::Shared(_), LockType::Exclusive) => {
                    // In a real impl this would block; for testing we fail
                    Err(FsError::Conflict {
                        path: std::path::PathBuf::new(),
                    })
                }
                (LockState::Exclusive, _) => {
                    // In a real impl this would block; for testing we fail
                    Err(FsError::Conflict {
                        path: std::path::PathBuf::new(),
                    })
                }
            }
        }

        fn try_lock(&self, handle: Handle, lock_type: LockType) -> Result<bool, FsError> {
            if !self.valid_handles.read().unwrap().contains(&handle.0) {
                return Err(FsError::InvalidHandle { handle });
            }

            let mut locks = self.locks.write().unwrap();
            let state = locks.entry(handle.0).or_insert(LockState::Unlocked);

            match (*state, lock_type) {
                (LockState::Unlocked, LockType::Shared) => {
                    *state = LockState::Shared(1);
                    Ok(true)
                }
                (LockState::Unlocked, LockType::Exclusive) => {
                    *state = LockState::Exclusive;
                    Ok(true)
                }
                (LockState::Shared(n), LockType::Shared) => {
                    *state = LockState::Shared(n + 1);
                    Ok(true)
                }
                (LockState::Shared(_), LockType::Exclusive) => Ok(false),
                (LockState::Exclusive, _) => Ok(false),
            }
        }

        fn unlock(&self, handle: Handle) -> Result<(), FsError> {
            if !self.valid_handles.read().unwrap().contains(&handle.0) {
                return Err(FsError::InvalidHandle { handle });
            }

            let mut locks = self.locks.write().unwrap();
            let state = locks.entry(handle.0).or_insert(LockState::Unlocked);

            match *state {
                LockState::Unlocked => Ok(()),
                LockState::Shared(1) => {
                    *state = LockState::Unlocked;
                    Ok(())
                }
                LockState::Shared(n) => {
                    *state = LockState::Shared(n - 1);
                    Ok(())
                }
                LockState::Exclusive => {
                    *state = LockState::Unlocked;
                    Ok(())
                }
            }
        }
    }

    #[test]
    fn lock_shared_succeeds() {
        let fs = MockLockFs::new();
        fs.add_handle(1);

        fs.lock(Handle(1), LockType::Shared).unwrap();
    }

    #[test]
    fn lock_exclusive_succeeds() {
        let fs = MockLockFs::new();
        fs.add_handle(1);

        fs.lock(Handle(1), LockType::Exclusive).unwrap();
    }

    #[test]
    fn lock_invalid_handle_fails() {
        let fs = MockLockFs::new();
        let result = fs.lock(Handle(999), LockType::Shared);
        assert!(matches!(result, Err(FsError::InvalidHandle { .. })));
    }

    #[test]
    fn multiple_shared_locks_succeed() {
        let fs = MockLockFs::new();
        fs.add_handle(1);

        fs.lock(Handle(1), LockType::Shared).unwrap();
        fs.lock(Handle(1), LockType::Shared).unwrap();
        fs.lock(Handle(1), LockType::Shared).unwrap();
    }

    #[test]
    fn try_lock_returns_false_when_blocked() {
        let fs = MockLockFs::new();
        fs.add_handle(1);

        fs.lock(Handle(1), LockType::Exclusive).unwrap();
        let result = fs.try_lock(Handle(1), LockType::Shared).unwrap();
        assert!(!result);
    }

    #[test]
    fn unlock_releases_lock() {
        let fs = MockLockFs::new();
        fs.add_handle(1);

        fs.lock(Handle(1), LockType::Exclusive).unwrap();
        fs.unlock(Handle(1)).unwrap();

        // Can now acquire again
        let result = fs.try_lock(Handle(1), LockType::Exclusive).unwrap();
        assert!(result);
    }

    #[test]
    fn unlock_invalid_handle_fails() {
        let fs = MockLockFs::new();
        let result = fs.unlock(Handle(999));
        assert!(matches!(result, Err(FsError::InvalidHandle { .. })));
    }
}
