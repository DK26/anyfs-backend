//! # Marker Traits
//!
//! Zero-cost marker traits for compile-time behavior selection.
//!
//! ## Overview
//!
//! Marker traits are empty traits that signal compile-time properties
//! without adding runtime overhead. They enable conditional behavior
//! through trait bounds.
//!
//! ## Available Markers
//!
//! | Marker | Purpose |
//! |--------|---------|
//! | [`SelfResolving`] | Backend handles its own path resolution |

/// Marker trait for backends that handle their own path resolution.
///
/// When a backend implements `SelfResolving`, the `FileStorage` wrapper
/// (in the `anyfs` crate) will NOT perform virtual path resolution.
/// Instead, paths are passed directly to the backend unchanged.
///
/// # Path Resolution
///
/// Path resolution involves:
/// - Normalizing `.` and `..` components
/// - Following symbolic links
/// - Resolving relative paths to absolute
///
/// # When to Implement
///
/// Implement `SelfResolving` for backends that delegate to an underlying
/// system that already handles path resolution:
///
/// - **`NativeBackend`**: Delegates to `std::fs`, OS handles resolution
/// - **`VRootFsBackend`**: Uses `strict-path` for containment, OS resolves
///
/// # When NOT to Implement
///
/// Do NOT implement for backends that store symlinks as data:
///
/// - **`MemoryBackend`**: Stores symlinks as data, needs software resolution
/// - **`SqliteBackend`**: Stores symlinks in database, needs software resolution
/// - **`OverlayBackend`**: Needs to resolve across layers
///
/// # How FileStorage Uses This
///
/// ```text
/// FileStorage<B> where B: SelfResolving
///     → Passes paths directly to backend
///
/// FileStorage<B> where B: !SelfResolving
///     → Resolves paths component-by-component using PathResolver
/// ```
///
/// # Example
///
/// ```rust
/// use anyfs_backend::SelfResolving;
/// use std::path::PathBuf;
///
/// /// A backend that wraps native filesystem access.
/// /// The OS handles all path resolution.
/// struct NativeBackend {
///     root: PathBuf,
/// }
///
/// // Mark as self-resolving - FileStorage won't do path resolution
/// impl SelfResolving for NativeBackend {}
/// ```
///
/// # Thread Safety
///
/// This is a marker trait with no methods, so thread safety is inherited
/// from the implementing type. Most backends are `Send + Sync`.
///
/// # Design Rationale
///
/// Using a marker trait instead of a runtime flag:
/// - **Zero cost**: No runtime checks, branch prediction, or vtables
/// - **Type safety**: Compile-time verification of path handling
/// - **Documentation**: Self-documenting code through trait bounds
pub trait SelfResolving {}

// Note: No blanket implementation - backends must explicitly opt-in
// by implementing this marker trait.

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn self_resolving_is_implementable() {
        struct TestBackend {
            _root: PathBuf,
        }

        impl SelfResolving for TestBackend {}

        let backend = TestBackend {
            _root: PathBuf::from("/test"),
        };

        // Verify the marker trait is implemented
        fn _check<T: SelfResolving>(_: &T) {}
        _check(&backend);
    }

    #[test]
    fn self_resolving_is_send_sync_compatible() {
        struct ThreadSafeBackend;

        impl SelfResolving for ThreadSafeBackend {}

        // SelfResolving doesn't require Send+Sync, but common backends are
        fn _check_send<T: Send>() {}
        fn _check_sync<T: Sync>() {}

        _check_send::<ThreadSafeBackend>();
        _check_sync::<ThreadSafeBackend>();
    }

    #[test]
    fn can_use_in_trait_bounds() {
        struct MockBackend;
        impl SelfResolving for MockBackend {}

        // Function that only accepts self-resolving backends
        fn process_self_resolving<B: SelfResolving>(_backend: &B) -> bool {
            true
        }

        let backend = MockBackend;
        assert!(process_self_resolving(&backend));
    }

    #[test]
    fn negative_bound_simulation() {
        // We can't use negative trait bounds in stable Rust,
        // but we can use specialization patterns or separate functions

        struct VirtualBackend; // Does NOT implement SelfResolving

        // This function works for any backend (whether SelfResolving or not)
        fn process_any<B>(_backend: &B) -> &'static str {
            "processed"
        }

        let backend = VirtualBackend;
        assert_eq!(process_any(&backend), "processed");

        // The distinction happens at the FileStorage level in the anyfs crate
    }
}
