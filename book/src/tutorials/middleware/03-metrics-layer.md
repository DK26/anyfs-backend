# Metrics Layer

A metrics layer collects statistics about filesystem usage:

- Operation counts
- Bytes read/written
- Error counts
- Latency histograms

## Design

Use atomic counters for thread safety:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct Metrics {
    pub reads: AtomicU64,
    pub writes: AtomicU64,
    pub bytes_read: AtomicU64,
    pub bytes_written: AtomicU64,
    pub errors: AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn summary(&self) -> String {
        format!(
            "reads={}, writes={}, bytes_read={}, bytes_written={}, errors={}",
            self.reads.load(Ordering::Relaxed),
            self.writes.load(Ordering::Relaxed),
            self.bytes_read.load(Ordering::Relaxed),
            self.bytes_written.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
}
```

## Implementation

```rust
use anyfs_backend::{Layer, FsRead, FsWrite, FsDir, FsError, Metadata, ReadDirIter};
use std::path::Path;

pub struct MetricsLayer {
    metrics: Arc<Metrics>,
}

impl MetricsLayer {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }
}

pub struct MetricsFs<B> {
    inner: B,
    metrics: Arc<Metrics>,
}

impl<B> Layer<B> for MetricsLayer {
    type Wrapped = MetricsFs<B>;

    fn layer(self, inner: B) -> Self::Wrapped {
        MetricsFs {
            inner,
            metrics: self.metrics,
        }
    }
}
```

### Counting FsRead

```rust
impl<B: FsRead> FsRead for MetricsFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);
        
        match self.inner.read(path) {
            Ok(data) => {
                self.metrics.bytes_read
                    .fetch_add(data.len() as u64, Ordering::Relaxed);
                Ok(data)
            }
            Err(e) => {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        self.inner.metadata(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        self.inner.exists(path)
    }
}
```

### Counting FsWrite

```rust
impl<B: FsWrite> FsWrite for MetricsFs<B> {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);
        
        match self.inner.write(path, content) {
            Ok(()) => {
                self.metrics.bytes_written
                    .fetch_add(content.len() as u64, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.inner.remove_file(path)
    }
}
```

### Forwarding FsDir

```rust
impl<B: FsDir> FsDir for MetricsFs<B> {
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

## Usage

```rust
use anyfs_backend::LayerExt;

// Create shared metrics
let metrics = Metrics::new();

let fs = InMemoryFs::new()
    .layer(MetricsLayer::new(metrics.clone()));

// Use the filesystem
fs.write(Path::new("/a.txt"), b"Hello").unwrap();
fs.write(Path::new("/b.txt"), b"World!").unwrap();
fs.read(Path::new("/a.txt")).unwrap();
fs.read(Path::new("/b.txt")).unwrap();
fs.read(Path::new("/a.txt")).unwrap();

// Check metrics
println!("{}", metrics.summary());
// Output: reads=3, writes=2, bytes_read=16, bytes_written=11, errors=0
```

## Advanced Metrics

### Latency Tracking

```rust
use std::time::{Duration, Instant};
use std::sync::RwLock;

pub struct DetailedMetrics {
    pub reads: AtomicU64,
    pub read_latencies: RwLock<Vec<Duration>>,  // For percentile calculations
}

impl<B: FsRead> FsRead for MetricsFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let start = Instant::now();
        let result = self.inner.read(path);
        let elapsed = start.elapsed();
        
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);
        self.metrics.read_latencies.write().unwrap().push(elapsed);
        
        result
    }
}
```

### Per-Path Metrics

```rust
use std::collections::HashMap;

pub struct PathMetrics {
    pub by_path: RwLock<HashMap<PathBuf, u64>>,
}

impl<B: FsRead> FsRead for PathMetricsFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let result = self.inner.read(path);
        
        if result.is_ok() {
            let mut by_path = self.metrics.by_path.write().unwrap();
            *by_path.entry(path.to_path_buf()).or_default() += 1;
        }
        
        result
    }
}
```

### Prometheus/OpenTelemetry Integration

```rust
// Pseudo-code for real metrics systems
pub struct PrometheusMetrics {
    reads: prometheus::Counter,
    bytes_read: prometheus::Counter,
    read_duration: prometheus::Histogram,
}

impl<B: FsRead> FsRead for PrometheusFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let timer = self.metrics.read_duration.start_timer();
        let result = self.inner.read(path);
        timer.observe_duration();
        
        self.metrics.reads.inc();
        if let Ok(data) = &result {
            self.metrics.bytes_read.inc_by(data.len() as f64);
        }
        
        result
    }
}
```

## Key Points

1. **Use atomics** for thread-safe counting
2. **Share metrics** via `Arc` to read from outside
3. **Count before/after** for accurate error tracking
4. **Consider scope** - global vs per-path vs per-operation

Next: [Caching Layer →](./04-caching-layer.md)
