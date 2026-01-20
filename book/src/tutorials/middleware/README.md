# Implementing Middleware

This tutorial teaches you how to create middleware layers that wrap filesystem backends to add cross-cutting functionality.

## What You'll Learn

- The Layer pattern (inspired by Tower)
- How to wrap any backend with additional behavior
- Creating logging, metrics, caching, and access control layers
- Composing multiple layers together

## Prerequisites

- Completed the [backend tutorial](../backend/README.md) or understand the trait hierarchy
- Familiarity with Rust traits and generics

## The Layer Pattern

Middleware wraps a backend to intercept operations:

```
              Request
                 │
                 ▼
     ┌─────────────────────┐
     │   Logging Layer     │ ← Logs all operations
     └─────────────────────┘
                 │
                 ▼
     ┌─────────────────────┐
     │   Caching Layer     │ ← Serves from cache
     └─────────────────────┘
                 │
                 ▼
     ┌─────────────────────┐
     │   Actual Backend    │ ← Real filesystem
     └─────────────────────┘
                 │
                 ▼
             Response
```

## Tutorial Structure

1. **[The Layer Pattern](./01-layer-pattern.md)** - Understanding the Layer trait
2. **[Logging Layer](./02-logging-layer.md)** - Log all operations
3. **[Metrics Layer](./03-metrics-layer.md)** - Collect statistics
4. **[Caching Layer](./04-caching-layer.md)** - Cache read results
5. **[Access Control Layer](./05-access-control.md)** - Restrict operations
6. **[Composing Layers](./06-composing-layers.md)** - Stack layers together

## Quick Example

```rust
use anyfs_backend::{Fs, Layer};

// Create a backend
let fs = InMemoryFs::new();

// Wrap with logging
let fs = LoggingLayer::new("MyApp").layer(fs);

// Wrap with caching
let fs = CachingLayer::new(Duration::from_secs(60)).layer(fs);

// Use normally - logging and caching are automatic
fs.write(Path::new("/file.txt"), b"Hello").unwrap();
let data = fs.read(Path::new("/file.txt")).unwrap();  // Cached!
```

Let's start with [The Layer Pattern →](./01-layer-pattern.md)
