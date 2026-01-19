# Composing Layers

The power of layers comes from composition. Stack multiple layers to combine functionality.

## Basic Composition

```rust
use anyfs_backend::LayerExt;

let fs = InMemoryFs::new()
    .layer(CachingLayer::new(Duration::from_secs(60)))
    .layer(MetricsLayer::new(metrics.clone()))
    .layer(LoggingLayer::new("FS"));
```

## Understanding Layer Order

When you compose layers:

```rust
backend.layer(A).layer(B).layer(C)
```

The result is: **C wraps B wraps A wraps backend**

```
Request → C → B → A → Backend → A → B → C → Response
          ↓   ↓   ↓            ↑   ↑   ↑
        (1) (2) (3)          (3) (2) (1)
```

**Outer layers see requests first and responses last.**

## Order Matters!

### Example: Logging + Caching

```rust
// Option 1: Logging outside Caching
let fs = backend
    .layer(CachingLayer::new(ttl))
    .layer(LoggingLayer::new("FS"));

// Log shows: read /file.txt (even for cache hits)
// Because logging is outside, it sees ALL requests
```

```rust
// Option 2: Caching outside Logging
let fs = backend
    .layer(LoggingLayer::new("FS"))
    .layer(CachingLayer::new(ttl));

// Log shows: read /file.txt (only for cache misses)
// Because cache handles request before it reaches logging
```

### Example: Metrics + Caching

```rust
// Metrics outside Caching - counts ALL reads
let fs = backend
    .layer(CachingLayer::new(ttl))
    .layer(MetricsLayer::new(m.clone()));
// metrics.reads = 100 (total requests)

// Caching outside Metrics - counts only cache misses
let fs = backend
    .layer(MetricsLayer::new(m.clone()))
    .layer(CachingLayer::new(ttl));
// metrics.reads = 10 (backend hits only)
```

## Recommended Layer Order

From innermost to outermost:

```rust
let fs = backend
    // 1. Transformations (encryption, compression)
    .layer(EncryptionLayer::new(key))
    
    // 2. Caching (after transformation)
    .layer(CachingLayer::new(Duration::from_secs(60)))
    
    // 3. Retry/resilience
    .layer(RetryLayer::new(3))
    
    // 4. Metrics (count actual operations)
    .layer(MetricsLayer::new(metrics.clone()))
    
    // 5. Logging (see everything)
    .layer(LoggingLayer::new("FS"))
    
    // 6. Access control (reject early)
    .layer(AccessControlLayer::new(rules));
```

Reasoning:
- **Encryption** must wrap raw backend to encrypt all data
- **Caching** stores encrypted data (or plaintext, depending on requirements)
- **Retry** retries failed operations
- **Metrics** count operations that reach this point
- **Logging** logs everything including rejections
- **Access control** rejects unauthorized requests immediately

## Type Complexity

Each layer adds a wrapper type:

```rust
let fs: LoggingFs<MetricsFs<CachingFs<InMemoryFs>>> = ...;
```

This can get unwieldy. Solutions:

### 1. Type Alias

```rust
type MyFs = LoggingFs<MetricsFs<CachingFs<InMemoryFs>>>;

fn create_fs() -> MyFs {
    InMemoryFs::new()
        .layer(CachingLayer::new(ttl))
        .layer(MetricsLayer::new(metrics))
        .layer(LoggingLayer::new("FS"))
}
```

### 2. Box with dyn Fs

```rust
fn create_fs() -> Box<dyn Fs> {
    let fs = InMemoryFs::new()
        .layer(CachingLayer::new(ttl))
        .layer(MetricsLayer::new(metrics))
        .layer(LoggingLayer::new("FS"));
    
    Box::new(fs)
}
```

### 3. impl Trait

```rust
fn create_fs() -> impl Fs {
    InMemoryFs::new()
        .layer(CachingLayer::new(ttl))
        .layer(MetricsLayer::new(metrics))
        .layer(LoggingLayer::new("FS"))
}
```

## Runtime Composition

For dynamic layer selection:

```rust
fn create_fs(config: &Config) -> Box<dyn Fs> {
    let mut fs: Box<dyn Fs> = Box::new(InMemoryFs::new());
    
    if config.enable_caching {
        fs = Box::new(CachingLayer::new(config.cache_ttl).layer(fs));
    }
    
    if config.enable_logging {
        fs = Box::new(LoggingLayer::new(&config.log_prefix).layer(fs));
    }
    
    if config.read_only {
        fs = Box::new(ReadOnlyLayer.layer(fs));
    }
    
    fs
}
```

Note: This requires layers to work with `Box<dyn Fs>`, which means implementing traits for the boxed type.

## Complete Example

```rust
use anyfs_backend::{Fs, LayerExt};
use std::path::Path;
use std::time::Duration;

fn main() {
    // Create shared metrics
    let metrics = Metrics::new();
    
    // Build the layered filesystem
    let fs = InMemoryFs::new()
        .layer(CachingLayer::new(Duration::from_secs(60)))
        .layer(MetricsLayer::new(metrics.clone()))
        .layer(LoggingLayer::new("APP"));
    
    // Setup
    fs.create_dir(Path::new("/data")).unwrap();
    fs.write(Path::new("/data/config.json"), b"{}").unwrap();
    
    // Multiple reads - watch cache behavior
    for i in 0..5 {
        let _ = fs.read(Path::new("/data/config.json"));
        println!("After read {}: {}", i + 1, metrics.summary());
    }
    
    // Output shows:
    // - All 5 reads logged (logging is outermost)
    // - Only 1 read in metrics (cache handles the rest)
}
```

## Testing Layered Systems

```rust
#[test]
fn test_layer_composition() {
    let metrics = Metrics::new();
    
    let fs = InMemoryFs::new()
        .layer(CachingLayer::new(Duration::from_secs(60)))
        .layer(MetricsLayer::new(metrics.clone()));
    
    fs.write(Path::new("/test.txt"), b"data").unwrap();
    
    // First read - cache miss, hits metrics
    fs.read(Path::new("/test.txt")).unwrap();
    assert_eq!(metrics.reads.load(Ordering::Relaxed), 1);
    
    // Second read - cache hit, doesn't hit metrics
    fs.read(Path::new("/test.txt")).unwrap();
    assert_eq!(metrics.reads.load(Ordering::Relaxed), 1);  // Still 1!
    
    // Write invalidates cache
    fs.write(Path::new("/test.txt"), b"new").unwrap();
    
    // Next read - cache miss again
    fs.read(Path::new("/test.txt")).unwrap();
    assert_eq!(metrics.reads.load(Ordering::Relaxed), 2);
}
```

## Summary

1. **Compose with `.layer()`** - Clean, fluent API
2. **Order matters** - Outer layers see requests first
3. **Think about what each layer should see** - Metrics before or after cache?
4. **Use type aliases or `impl Trait`** - Manage type complexity
5. **Test the composition** - Verify layers interact correctly

🎉 **Congratulations!** You now know how to:
- Create middleware layers
- Log, measure, cache, and control access
- Compose layers for powerful, reusable functionality

Go build something awesome!
