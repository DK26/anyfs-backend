# Logging Layer

A logging layer prints information about each filesystem operation. Useful for:

- Debugging
- Auditing
- Understanding access patterns

## Design

```rust
pub struct LoggingLayer {
    prefix: String,  // Prefix for log messages
}

pub struct LoggingFs<B> {
    inner: B,
    prefix: String,
}
```

## Implementation

```rust
use anyfs_backend::{Layer, FsRead, FsWrite, FsDir, FsError, Metadata, ReadDirIter};
use std::path::Path;
use std::time::Instant;

pub struct LoggingLayer {
    prefix: String,
}

impl LoggingLayer {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self { prefix: prefix.into() }
    }
}

pub struct LoggingFs<B> {
    inner: B,
    prefix: String,
}

impl<B> Layer<B> for LoggingLayer {
    type Wrapped = LoggingFs<B>;

    fn layer(self, inner: B) -> Self::Wrapped {
        LoggingFs {
            inner,
            prefix: self.prefix,
        }
    }
}
```

### Logging FsRead

```rust
impl<B: FsRead> FsRead for LoggingFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let start = Instant::now();
        println!("[{}] read: {}", self.prefix, path.display());
        
        let result = self.inner.read(path);
        let elapsed = start.elapsed();
        
        match &result {
            Ok(data) => println!(
                "[{}] read: {} → {} bytes ({:?})",
                self.prefix, path.display(), data.len(), elapsed
            ),
            Err(e) => println!(
                "[{}] read: {} → ERROR: {} ({:?})",
                self.prefix, path.display(), e, elapsed
            ),
        }
        
        result
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        println!("[{}] metadata: {}", self.prefix, path.display());
        self.inner.metadata(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        let result = self.inner.exists(path);
        println!("[{}] exists: {} → {:?}", self.prefix, path.display(), result);
        result
    }
}
```

### Logging FsWrite

```rust
impl<B: FsWrite> FsWrite for LoggingFs<B> {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        println!(
            "[{}] write: {} ({} bytes)",
            self.prefix, path.display(), content.len()
        );
        
        let result = self.inner.write(path, content);
        
        match &result {
            Ok(()) => println!("[{}] write: {} → OK", self.prefix, path.display()),
            Err(e) => println!("[{}] write: {} → ERROR: {}", self.prefix, path.display(), e),
        }
        
        result
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        println!("[{}] remove_file: {}", self.prefix, path.display());
        self.inner.remove_file(path)
    }
}
```

### Logging FsDir

```rust
impl<B: FsDir> FsDir for LoggingFs<B> {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        println!("[{}] read_dir: {}", self.prefix, path.display());
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        println!("[{}] create_dir: {}", self.prefix, path.display());
        self.inner.create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        println!("[{}] create_dir_all: {}", self.prefix, path.display());
        self.inner.create_dir_all(path)
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        println!("[{}] remove_dir: {}", self.prefix, path.display());
        self.inner.remove_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        println!("[{}] remove_dir_all: {}", self.prefix, path.display());
        self.inner.remove_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        println!(
            "[{}] rename: {} → {}",
            self.prefix, from.display(), to.display()
        );
        self.inner.rename(from, to)
    }
}
```

## Usage

```rust
use anyfs_backend::LayerExt;

let fs = InMemoryFs::new()
    .layer(LoggingLayer::new("FS"));

fs.create_dir(Path::new("/docs")).unwrap();
fs.write(Path::new("/docs/readme.txt"), b"Hello").unwrap();
let _ = fs.read(Path::new("/docs/readme.txt")).unwrap();
```

Output:

```
[FS] create_dir: /docs
[FS] write: /docs/readme.txt (5 bytes)
[FS] write: /docs/readme.txt → OK
[FS] read: /docs/readme.txt
[FS] read: /docs/readme.txt → 5 bytes (45µs)
```

## Variations

### Log Level Support

```rust
pub struct LoggingLayer {
    prefix: String,
    level: LogLevel,
}

pub enum LogLevel {
    Debug,
    Info,
    Warn,
}

impl<B: FsRead> FsRead for LoggingFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        if self.level <= LogLevel::Debug {
            println!("[{}] read: {}", self.prefix, path.display());
        }
        // ...
    }
}
```

### Log to File/Custom Logger

```rust
use std::sync::Arc;

pub trait Logger: Send + Sync {
    fn log(&self, message: &str);
}

pub struct LoggingLayer<L> {
    logger: Arc<L>,
}

pub struct LoggingFs<B, L> {
    inner: B,
    logger: Arc<L>,
}
```

### Filter by Path

```rust
pub struct LoggingLayer {
    prefix: String,
    filter: Option<PathBuf>,  // Only log operations under this path
}
```

## Key Points

1. **Log before and after** - Shows timing and results
2. **Include context** - Path, size, duration
3. **Log errors** - Don't suppress, just log and forward
4. **Configurable** - Prefix, level, filter

Next: [Metrics Layer →](./03-metrics-layer.md)
