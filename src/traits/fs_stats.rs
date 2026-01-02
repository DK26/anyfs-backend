//! Filesystem statistics operations.

use crate::{FsError, StatFs};

/// Filesystem statistics operations.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync`. Methods use `&self` to allow
/// concurrent access.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `dyn FsStats`.
pub trait FsStats: Send + Sync {
    /// Get filesystem-level statistics.
    ///
    /// Returns information about total/used/available space and inodes.
    ///
    /// # Errors
    ///
    /// - [`FsError::Backend`] for backend-specific failures
    fn statfs(&self) -> Result<StatFs, FsError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_stats_is_object_safe() {
        fn _check(_: &dyn FsStats) {}
    }

    #[test]
    fn fs_stats_requires_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        fn _check<T: FsStats>() {
            _assert_send_sync::<T>();
        }
    }
}
