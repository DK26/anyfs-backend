# Caching Layer

A caching layer stores read results to avoid repeated backend access. Essential for:

- Remote backends (S3, network filesystems)
- Slow storage
- Reducing API calls/costs

## Design Decisions

1. **What to cache**: File contents, metadata, or both
2. **TTL**: How long entries remain valid
3. **Invalidation**: When to remove stale entries
4. **Size limits**: Maximum cache size (LRU eviction)

## Basic Implementation

```rust
use anyfs_backend::{Layer, FsRead, FsWrite, FsDir, FsError, Metadata, ReadDirIter};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub struct CachingLayer {
    ttl: Duration,
}

impl CachingLayer {
    pub fn new(ttl: Duration) -> Self {
        Self { ttl }
    }
}

struct CacheEntry {
    data: Vec<u8>,
    expires_at: Instant,
}

pub struct CachingFs<B> {
    inner: B,
    cache: RwLock<HashMap<PathBuf, CacheEntry>>,
    ttl: Duration,
}

impl<B> Layer<B> for CachingLayer {
    type Wrapped = CachingFs<B>;

    fn layer(self, inner: B) -> Self::Wrapped {
        CachingFs {
            inner,
            cache: RwLock::new(HashMap::new()),
            ttl: self.ttl,
        }
    }
}
```

### Caching Reads

```rust
impl<B: FsRead> FsRead for CachingFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        let path_buf = path.to_path_buf();

        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(&path_buf) {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.data.clone());  // Cache hit!
                }
            }
        }

        // Cache miss - read from backend
        let data = self.inner.read(path)?;

        // Store in cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(path_buf, CacheEntry {
                data: data.clone(),
                expires_at: Instant::now() + self.ttl,
            });
        }

        Ok(data)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        // Could cache metadata too
        self.inner.metadata(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.inner.exists(path)
    }
}
```

### Invalidating on Write

**Critical**: Writes must invalidate cached entries!

```rust
impl<B: FsWrite> FsWrite for CachingFs<B> {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        // Invalidate cache entry BEFORE writing
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(path);
        }

        self.inner.write(path, content)
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(path);
        }

        self.inner.remove_file(path)
    }
}
```

### Handling Directory Operations

```rust
impl<B: FsDir> FsDir for CachingFs<B> {
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
        // Invalidate ALL entries under this path
        {
            let mut cache = self.cache.write().unwrap();
            cache.retain(|p, _| !p.starts_with(path));
        }

        self.inner.remove_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        // Invalidate both paths
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(from);
            cache.remove(to);
        }

        self.inner.rename(from, to)
    }
}
```

## Usage

```rust
use anyfs_backend::LayerExt;
use std::time::Duration;

let fs = InMemoryFs::new()
    .layer(CachingLayer::new(Duration::from_secs(60)));

// First read - cache miss, reads from backend
let data1 = fs.read(Path::new("/file.txt")).unwrap();

// Second read - cache hit, returns cached data
let data2 = fs.read(Path::new("/file.txt")).unwrap();

// After write, cache is invalidated
fs.write(Path::new("/file.txt"), b"new content").unwrap();

// This read fetches fresh data
let data3 = fs.read(Path::new("/file.txt")).unwrap();
```

## Advanced Features

### LRU Eviction

```rust
use std::collections::VecDeque;

struct LruCache {
    entries: HashMap<PathBuf, CacheEntry>,
    order: VecDeque<PathBuf>,  // Oldest first
    max_size: usize,
}

impl LruCache {
    fn insert(&mut self, path: PathBuf, data: Vec<u8>, ttl: Duration) {
        // Evict if at capacity
        while self.entries.len() >= self.max_size {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        
        self.entries.insert(path.clone(), CacheEntry {
            data,
            expires_at: Instant::now() + ttl,
        });
        self.order.push_back(path);
    }
}
```

### Size-Limited Cache

```rust
struct SizeAwareCache {
    entries: HashMap<PathBuf, CacheEntry>,
    total_bytes: usize,
    max_bytes: usize,
}

impl SizeAwareCache {
    fn insert(&mut self, path: PathBuf, data: Vec<u8>, ttl: Duration) {
        let size = data.len();
        
        // Don't cache if too large
        if size > self.max_bytes / 10 {
            return;
        }
        
        // Evict until space available
        while self.total_bytes + size > self.max_bytes {
            // Evict oldest/smallest/least-used
        }
        
        self.total_bytes += size;
        self.entries.insert(path, CacheEntry { data, expires_at: ... });
    }
}
```

### Negative Caching

Cache "not found" results to avoid repeated lookups:

```rust
enum CachedResult {
    Found(Vec<u8>),
    NotFound,
}

fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
    if let Some(cached) = self.get_cached(path) {
        match cached {
            CachedResult::Found(data) => return Ok(data),
            CachedResult::NotFound => return Err(FsError::NotFound { ... }),
        }
    }
    
    match self.inner.read(path) {
        Ok(data) => {
            self.cache(path, CachedResult::Found(data.clone()));
            Ok(data)
        }
        Err(FsError::NotFound { .. }) => {
            self.cache(path, CachedResult::NotFound);
            Err(FsError::NotFound { path: path.to_path_buf() })
        }
        Err(e) => Err(e),
    }
}
```

## Cache Consistency

### Strong Consistency

Invalidate on every write operation. Safe but may miss external changes.

### Eventual Consistency

Use short TTLs. Allows stale reads but simpler.

### Write-Through

Update cache on write instead of invalidating:

```rust
fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
    self.inner.write(path, content)?;
    
    // Update cache with new content
    let mut cache = self.cache.write().unwrap();
    cache.insert(path.to_path_buf(), CacheEntry {
        data: content.to_vec(),
        expires_at: Instant::now() + self.ttl,
    });
    
    Ok(())
}
```

## Key Points

1. **Always invalidate on writes** - Stale cache is worse than no cache
2. **Consider TTL carefully** - Too long = stale data, too short = no benefit
3. **Handle directory operations** - `remove_dir_all` affects many paths
4. **Memory limits** - Unbounded caches can cause OOM

Next: [Access Control Layer →](./05-access-control.md)
