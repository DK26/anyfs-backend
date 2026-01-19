//! # Layer Trait
//!
//! Tower-style middleware composition for filesystem backends.
//!
//! ## Overview
//!
//! The [`Layer`] trait enables composable middleware that wraps backends to add
//! functionality like caching, encryption, rate limiting, or logging.
//!
//! ## How It Works
//!
//! ```text
//! Backend ──▶ Layer::layer() ──▶ Wrapped Backend
//! ```
//!
//! Each middleware provides:
//! 1. A wrapper struct that implements filesystem traits
//! 2. A `Layer` implementation that creates the wrapper
//!
//! ## Example
//!
//! The Layer pattern separates middleware configuration from wrapping:
//!
//! ```rust
//! use anyfs_backend::Layer;
//!
//! // Configuration for the layer
//! struct CacheConfig {
//!     max_entries: usize,
//! }
//!
//! // The layer holds configuration
//! struct CacheLayer {
//!     config: CacheConfig,
//! }
//!
//! // The middleware wraps any backend
//! struct CacheMiddleware<B> {
//!     inner: B,
//!     config: CacheConfig,
//! }
//!
//! // Layer creates the middleware
//! impl<B> Layer<B> for CacheLayer {
//!     type Backend = CacheMiddleware<B>;
//!     
//!     fn layer(self, backend: B) -> Self::Backend {
//!         CacheMiddleware {
//!             inner: backend,
//!             config: self.config,
//!         }
//!     }
//! }
//! ```
//!
//! ## Fluent Composition
//!
//! Use [`LayerExt`] for fluent chaining:
//!
//! ```rust
//! use anyfs_backend::LayerExt;
//!
//! // Hypothetical usage (requires concrete backend):
//! // let backend = MemoryBackend::new()
//! //     .layer(QuotaLayer::new(limits))
//! //     .layer(TracingLayer::new());
//! ```

use crate::Fs;

/// A layer that wraps a backend to add functionality.
///
/// Inspired by Tower's `Layer` trait, this enables composable middleware.
/// Each middleware provides a corresponding `Layer` implementation.
///
/// # Type Parameters
///
/// - `B`: The backend type being wrapped (must implement [`Fs`])
///
/// # Design Notes
///
/// - `layer(self, backend)` consumes both the layer and backend
/// - The resulting `Backend` type must also implement `Fs`
/// - Middleware needing higher traits (e.g., `FsLink`) can add bounds in their impl
///
/// # Example
///
/// ```rust
/// use anyfs_backend::Layer;
///
/// struct LoggingMiddleware<B> {
///     inner: B,
///     prefix: String,
/// }
///
/// struct LoggingLayer {
///     prefix: String,
/// }
///
/// impl<B> Layer<B> for LoggingLayer {
///     type Backend = LoggingMiddleware<B>;
///     
///     fn layer(self, backend: B) -> Self::Backend {
///         LoggingMiddleware {
///             inner: backend,
///             prefix: self.prefix,
///         }
///     }
/// }
/// ```
pub trait Layer<B> {
    /// The resulting backend type after applying this layer.
    ///
    /// For middleware that preserves filesystem capabilities, this type
    /// should implement the same traits as the input backend `B`.
    type Backend;

    /// Wrap the given backend with this layer's functionality.
    ///
    /// Consumes both the layer configuration and the backend,
    /// returning a new wrapped backend.
    fn layer(self, backend: B) -> Self::Backend;
}

/// Extension trait for fluent layer composition.
///
/// Provides the `.layer()` method on any `Fs` backend for ergonomic chaining.
///
/// # Example
///
/// ```rust
/// use anyfs_backend::{Fs, LayerExt, Layer};
///
/// // With LayerExt, you can chain layers fluently:
/// fn compose_backend<B: Fs, L: Layer<B>>(backend: B, layer: L) -> L::Backend {
///     backend.layer(layer)
/// }
/// ```
pub trait LayerExt: Fs + Sized {
    /// Apply a layer to this backend.
    ///
    /// Returns the wrapped backend with the layer's functionality added.
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyfs_backend::{Fs, LayerExt, Layer};
    ///
    /// fn add_middleware<B, L>(backend: B, layer: L) -> L::Backend
    /// where
    ///     B: Fs,
    ///     L: Layer<B>,
    /// {
    ///     backend.layer(layer)
    /// }
    /// ```
    fn layer<L: Layer<Self>>(self, layer: L) -> L::Backend {
        layer.layer(self)
    }
}

// Blanket implementation - any Fs backend gets LayerExt for free
impl<B: Fs> LayerExt for B {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify Layer trait is object-safe (can be used as trait object)
    /// Note: Layer is NOT object-safe due to generic parameter and Sized bound
    /// This is intentional - layers are compile-time composition

    #[test]
    fn layer_ext_is_auto_implemented() {
        // LayerExt is blanket-implemented for all Fs types
        fn _check<B: Fs + LayerExt>() {}
    }

    #[test]
    fn layer_composes_types() {
        use crate::{FsDir, FsRead, FsWrite, ReadDirIter};
        use std::path::Path;

        // Mock backend
        struct MockBackend;

        impl FsRead for MockBackend {
            fn read(&self, _: &Path) -> Result<Vec<u8>, crate::FsError> {
                Ok(vec![])
            }
            fn read_to_string(&self, _: &Path) -> Result<String, crate::FsError> {
                Ok(String::new())
            }
            fn read_range(&self, _: &Path, _: u64, _: usize) -> Result<Vec<u8>, crate::FsError> {
                Ok(vec![])
            }
            fn exists(&self, _: &Path) -> Result<bool, crate::FsError> {
                Ok(true)
            }
            fn metadata(&self, _: &Path) -> Result<crate::Metadata, crate::FsError> {
                Ok(crate::Metadata::default())
            }
            fn open_read(&self, _: &Path) -> Result<Box<dyn std::io::Read + Send>, crate::FsError> {
                Ok(Box::new(std::io::empty()))
            }
        }

        impl FsWrite for MockBackend {
            fn write(&self, _: &Path, _: &[u8]) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn append(&self, _: &Path, _: &[u8]) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn truncate(&self, _: &Path, _: u64) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn remove_file(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn rename(&self, _: &Path, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn copy(&self, _: &Path, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn open_write(
                &self,
                _: &Path,
            ) -> Result<Box<dyn std::io::Write + Send>, crate::FsError> {
                Ok(Box::new(std::io::sink()))
            }
        }

        impl FsDir for MockBackend {
            fn read_dir(&self, _: &Path) -> Result<ReadDirIter, crate::FsError> {
                Ok(ReadDirIter::from_vec(vec![]))
            }
            fn create_dir(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn create_dir_all(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn remove_dir(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn remove_dir_all(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
        }

        // Mock wrapper
        struct WrappedBackend<B> {
            _inner: B,
        }

        impl<B: FsRead> FsRead for WrappedBackend<B> {
            fn read(&self, _: &Path) -> Result<Vec<u8>, crate::FsError> {
                Ok(vec![])
            }
            fn read_to_string(&self, _: &Path) -> Result<String, crate::FsError> {
                Ok(String::new())
            }
            fn read_range(&self, _: &Path, _: u64, _: usize) -> Result<Vec<u8>, crate::FsError> {
                Ok(vec![])
            }
            fn exists(&self, _: &Path) -> Result<bool, crate::FsError> {
                Ok(true)
            }
            fn metadata(&self, _: &Path) -> Result<crate::Metadata, crate::FsError> {
                Ok(crate::Metadata::default())
            }
            fn open_read(&self, _: &Path) -> Result<Box<dyn std::io::Read + Send>, crate::FsError> {
                Ok(Box::new(std::io::empty()))
            }
        }

        impl<B: FsWrite> FsWrite for WrappedBackend<B> {
            fn write(&self, _: &Path, _: &[u8]) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn append(&self, _: &Path, _: &[u8]) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn truncate(&self, _: &Path, _: u64) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn remove_file(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn rename(&self, _: &Path, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn copy(&self, _: &Path, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn open_write(
                &self,
                _: &Path,
            ) -> Result<Box<dyn std::io::Write + Send>, crate::FsError> {
                Ok(Box::new(std::io::sink()))
            }
        }

        impl<B: FsDir> FsDir for WrappedBackend<B> {
            fn read_dir(&self, _: &Path) -> Result<ReadDirIter, crate::FsError> {
                Ok(ReadDirIter::from_vec(vec![]))
            }
            fn create_dir(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn create_dir_all(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn remove_dir(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
            fn remove_dir_all(&self, _: &Path) -> Result<(), crate::FsError> {
                Ok(())
            }
        }

        // Mock layer
        struct MockLayer;

        impl<B: Fs> Layer<B> for MockLayer {
            type Backend = WrappedBackend<B>;

            fn layer(self, backend: B) -> Self::Backend {
                WrappedBackend { _inner: backend }
            }
        }

        // Test composition
        let backend = MockBackend;
        let wrapped = backend.layer(MockLayer);

        // Verify wrapped backend implements Fs
        fn _takes_fs<T: Fs>(_: &T) {}
        _takes_fs(&wrapped);
    }
}
