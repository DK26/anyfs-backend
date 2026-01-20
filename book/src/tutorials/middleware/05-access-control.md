# Access Control Layer

An access control layer restricts operations based on rules:

- Read-only mode
- Path restrictions
- User-based permissions

## Design

```rust
pub enum AccessRule {
    /// Deny all write operations
    ReadOnly,
    
    /// Only allow operations under a specific path
    RestrictToPath(PathBuf),
    
    /// Custom rule function
    Custom(Box<dyn Fn(&Path, Operation) -> bool + Send + Sync>),
}

pub enum Operation {
    Read,
    Write,
    Delete,
    List,
    Create,
}
```

## Read-Only Implementation

The simplest access control: block all writes.

```rust
use anyfs_backend::{Layer, FsRead, FsWrite, FsDir, FsError, Metadata, ReadDirIter};
use std::path::Path;

pub struct ReadOnlyLayer;

pub struct ReadOnlyFs<B> {
    inner: B,
}

impl<B> Layer<B> for ReadOnlyLayer {
    type Backend = ReadOnlyFs<B>;

    fn layer(self, inner: B) -> Self::Backend {
        ReadOnlyFs { inner }
    }
}

// Forward all reads unchanged
impl<B: FsRead> FsRead for ReadOnlyFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.inner.read(path)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        self.inner.read_to_string(path)
    }

    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        self.inner.read_range(path, offset, len)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        self.inner.metadata(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        self.inner.exists(path)
    }
}

// Block all writes
impl<B: FsWrite> FsWrite for ReadOnlyFs<B> {
    fn write(&self, _path: &Path, _content: &[u8]) -> Result<(), FsError> {
        Err(FsError::ReadOnly { operation: "write" })
    }

    fn remove_file(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::ReadOnly { operation: "remove_file" })
    }
}

impl<B: FsDir> FsDir for ReadOnlyFs<B> {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        self.inner.read_dir(path)  // Reading is allowed
    }

    fn create_dir(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::ReadOnly { operation: "create_dir" })
    }

    fn create_dir_all(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::ReadOnly { operation: "create_dir_all" })
    }

    fn remove_dir(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::ReadOnly { operation: "remove_dir" })
    }

    fn remove_dir_all(&self, _path: &Path) -> Result<(), FsError> {
        Err(FsError::ReadOnly { operation: "remove_dir_all" })
    }

    fn rename(&self, from: &Path, _to: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied { path: from.to_path_buf() })
    }
}
```

## Path-Restricted Implementation

Only allow access under certain paths:

```rust
use std::path::PathBuf;

pub struct PathRestrictedLayer {
    allowed_paths: Vec<PathBuf>,
}

impl PathRestrictedLayer {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { allowed_paths: paths }
    }
    
    pub fn single(path: impl Into<PathBuf>) -> Self {
        Self { allowed_paths: vec![path.into()] }
    }
}

pub struct PathRestrictedFs<B> {
    inner: B,
    allowed_paths: Vec<PathBuf>,
}

impl<B> PathRestrictedFs<B> {
    fn check_path(&self, path: &Path) -> Result<(), FsError> {
        for allowed in &self.allowed_paths {
            if path.starts_with(allowed) {
                return Ok(());
            }
        }
        Err(FsError::PermissionDenied { path: path.to_path_buf() })
    }
}

impl<B> Layer<B> for PathRestrictedLayer {
    type Wrapped = PathRestrictedFs<B>;

    fn layer(self, inner: B) -> Self::Wrapped {
        PathRestrictedFs {
            inner,
            allowed_paths: self.allowed_paths,
        }
    }
}

impl<B: FsRead> FsRead for PathRestrictedFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.check_path(path)?;
        self.inner.read(path)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        self.check_path(path)?;
        self.inner.metadata(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        if self.check_path(path).is_err() {
            return Ok(false);  // Pretend it doesn't exist
        }
        self.inner.exists(path)
    }
}

impl<B: FsWrite> FsWrite for PathRestrictedFs<B> {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        self.check_path(path)?;
        self.inner.write(path, content)
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.check_path(path)?;
        self.inner.remove_file(path)
    }
}

// FsDir implementation similar...
```

## Usage

### Read-Only Mode

```rust
use anyfs_backend::LayerExt;

// Populate the filesystem first
let fs = InMemoryFs::new();
fs.create_dir(Path::new("/data")).unwrap();
fs.write(Path::new("/data/file.txt"), b"content").unwrap();

// Now make it read-only
let fs = fs.layer(ReadOnlyLayer);

// Reading works
let data = fs.read(Path::new("/data/file.txt")).unwrap();

// Writing fails
match fs.write(Path::new("/data/new.txt"), b"test") {
    Err(FsError::PermissionDenied { .. }) => println!("Blocked!"),
    _ => panic!("Should have been blocked"),
}
```

### Path Restriction

```rust
let fs = InMemoryFs::new();
fs.create_dir_all(Path::new("/allowed/subdir")).unwrap();
fs.create_dir(Path::new("/forbidden")).unwrap();
fs.write(Path::new("/allowed/file.txt"), b"ok").unwrap();
fs.write(Path::new("/forbidden/secret.txt"), b"hidden").unwrap();

// Restrict to /allowed only
let fs = fs.layer(PathRestrictedLayer::single("/allowed"));

// This works
let data = fs.read(Path::new("/allowed/file.txt")).unwrap();

// This fails
match fs.read(Path::new("/forbidden/secret.txt")) {
    Err(FsError::PermissionDenied { .. }) => println!("Access denied!"),
    _ => panic!("Should have been denied"),
}
```

## Advanced: Custom Rules

```rust
pub struct CustomAccessLayer<F> {
    checker: F,
}

impl<F> CustomAccessLayer<F>
where
    F: Fn(&Path, Operation) -> bool + Send + Sync + Clone,
{
    pub fn new(checker: F) -> Self {
        Self { checker }
    }
}

// Example: Allow reads anywhere, writes only to /tmp
let fs = backend.layer(CustomAccessLayer::new(|path, op| {
    match op {
        Operation::Read | Operation::List => true,
        Operation::Write | Operation::Create | Operation::Delete => {
            path.starts_with("/tmp")
        }
    }
}));
```

## Combining with Other Layers

Access control should usually be the **outermost** layer:

```rust
let fs = InMemoryFs::new()
    .layer(CachingLayer::new(Duration::from_secs(60)))
    .layer(MetricsLayer::new(metrics.clone()))
    .layer(LoggingLayer::new("FS"))
    .layer(ReadOnlyLayer);  // Outermost - checked first

// Flow: ReadOnly -> Logging -> Metrics -> Caching -> Backend
```

This way:
1. Access control rejects unauthorized requests immediately
2. Logging sees the rejection
3. Metrics don't count blocked requests (if desired)

## Key Points

1. **Return `PermissionDenied`** for blocked operations
2. **Check early** - Don't do work before validating access
3. **Consider `exists()`** - Should forbidden paths appear to not exist?
4. **Layer order matters** - Put access control outermost

Next: [Composing Layers →](./06-composing-layers.md)
