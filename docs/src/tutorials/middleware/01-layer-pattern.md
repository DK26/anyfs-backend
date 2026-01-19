# The Layer Pattern

The Layer pattern allows you to wrap a backend with additional behavior without modifying the backend itself.

## The Layer Trait

```rust
pub trait Layer<B> {
    /// The resulting wrapped type.
    type Wrapped;

    /// Wrap the backend, producing a new type.
    fn layer(self, inner: B) -> Self::Wrapped;
}
```

- `B`: The inner backend type being wrapped
- `Wrapped`: The resulting wrapped type (must implement same traits)
- `layer()`: Consumes the layer config and backend, produces wrapped backend

## Basic Structure

Every layer needs two types:

1. **Layer type**: Configuration/factory (e.g., `LoggingLayer`)
2. **Wrapped type**: The actual wrapper (e.g., `LoggingFs<B>`)

```rust
/// Layer configuration (the factory)
pub struct MyLayer {
    // Configuration options
}

/// The wrapped backend
pub struct MyWrapper<B> {
    inner: B,
    // Layer state
}

impl<B> Layer<B> for MyLayer {
    type Wrapped = MyWrapper<B>;

    fn layer(self, inner: B) -> Self::Wrapped {
        MyWrapper {
            inner,
            // Initialize state from config
        }
    }
}
```

## Pass-Through Layer Example

The simplest layer does nothing—it just forwards all calls:

```rust
use anyfs_backend::{Layer, FsRead, FsWrite, FsDir, FsError, Metadata, ReadDirIter};
use std::path::Path;

/// A layer that does nothing.
pub struct PassThroughLayer;

/// Wraps any backend, forwarding all calls.
pub struct PassThrough<B> {
    inner: B,
}

impl<B> Layer<B> for PassThroughLayer {
    type Wrapped = PassThrough<B>;

    fn layer(self, inner: B) -> Self::Wrapped {
        PassThrough { inner }
    }
}

// Forward all trait methods to inner

impl<B: FsRead> FsRead for PassThrough<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.inner.read(path)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        self.inner.metadata(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.inner.exists(path)
    }
}

impl<B: FsWrite> FsWrite for PassThrough<B> {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        self.inner.write(path, content)
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.inner.remove_file(path)
    }
}

impl<B: FsDir> FsDir for PassThrough<B> {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        self.inner.create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        self.inner.create_dir_all(path)
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        self.inner.remove_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        self.inner.remove_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        self.inner.rename(from, to)
    }
}
```

## Why This Pattern?

### 1. Composition

Layers can be stacked:

```rust
let fs = backend
    .layer(LoggingLayer::new())
    .layer(CachingLayer::new())
    .layer(MetricsLayer::new());
```

### 2. Separation of Concerns

Each layer handles one thing:
- Logging layer: only logs
- Caching layer: only caches
- Metrics layer: only counts

### 3. Reusability

Write once, use with any backend:

```rust
let memory_fs = InMemoryFs::new().layer(LoggingLayer::new());
let disk_fs = DiskFs::new("/").layer(LoggingLayer::new());
let s3_fs = S3Fs::new(bucket).layer(LoggingLayer::new());
```

## The LayerExt Trait

For convenient chaining, use `LayerExt`:

```rust
use anyfs_backend::LayerExt;

// Instead of:
let fs = LoggingLayer::new().layer(backend);

// You can write:
let fs = backend.layer(LoggingLayer::new());
```

`LayerExt` is automatically implemented for all types.

## Trait Bounds

The wrapper only implements traits that the inner backend implements:

```rust
impl<B: FsRead> FsRead for MyWrapper<B> { ... }
//      ^^^^^^
//      Only if B implements FsRead
```

This means:
- Wrapping an `Fs` backend gives you an `Fs` wrapper
- Wrapping an `FsFull` backend gives you an `FsFull` wrapper
- The wrapper "inherits" the inner backend's capabilities

## Key Points

1. **Layer** = Configuration + Factory
2. **Wrapped** = The actual wrapper struct
3. **Forward traits** you want to preserve
4. **Add behavior** in the forwarding methods
5. **Use generics** to work with any backend

Next: [Logging Layer →](./02-logging-layer.md)
