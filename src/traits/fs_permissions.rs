//! Permission management operations.

use std::path::Path;

use crate::{FsError, Permissions};

/// Permission management operations.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync`. Methods use `&self` to allow
/// concurrent access.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `dyn FsPermissions`.
///
/// # Note
///
/// Reading permissions is done via [`FsRead::metadata`](super::FsRead::metadata).
/// This trait only provides the ability to set permissions.
pub trait FsPermissions: Send + Sync {
    /// Set permissions on a file or directory.
    ///
    /// # Errors
    ///
    /// - [`FsError::NotFound`] if the path does not exist
    /// - [`FsError::FeatureNotEnabled`] if blocked by `Restrictions` middleware
    fn set_permissions(&self, path: &Path, perm: Permissions) -> Result<(), FsError>;
}
