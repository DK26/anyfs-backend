# Quick Start

Add `anyfs-backend` to your `Cargo.toml`:

```toml
[dependencies]
anyfs-backend = "0.1"
```

## Using a Filesystem Backend

All backends implement the `Fs` trait (or higher-level traits). Write generic code against these traits:

```rust
use anyfs_backend::{Fs, FsError};
use std::path::Path;

fn list_files<B: Fs>(fs: &B, dir: &Path) -> Result<Vec<String>, FsError> {
    let mut names = Vec::new();
    for entry in fs.read_dir(dir)? {
        let entry = entry?;
        names.push(entry.name);
    }
    Ok(names)
}
```

## Creating a Simple Backend

Here's a minimal in-memory filesystem:

```rust
use anyfs_backend::{FsRead, FsWrite, FsDir, FsError, Metadata, DirEntry, FileType, Permissions, ReadDirIter};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub struct SimpleFs {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
}

impl SimpleFs {
    pub fn new() -> Self {
        Self { files: RwLock::new(HashMap::new()) }
    }
}

impl FsRead for SimpleFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.files.read().unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| FsError::NotFound { path: path.to_path_buf() })
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        let files = self.files.read().unwrap();
        let content = files.get(path)
            .ok_or_else(|| FsError::NotFound { path: path.to_path_buf() })?;
        
        Ok(Metadata {
            path: path.to_path_buf(),
            file_type: FileType::File,
            len: content.len() as u64,
            permissions: Permissions::default(),
            ..Default::default()
        })
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.read().unwrap().contains_key(path)
    }
}

impl FsWrite for SimpleFs {
    fn write(&self, path: &Path, content: &[u8]) -> Result<(), FsError> {
        self.files.write().unwrap().insert(path.to_path_buf(), content.to_vec());
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.files.write().unwrap()
            .remove(path)
            .map(|_| ())
            .ok_or_else(|| FsError::NotFound { path: path.to_path_buf() })
    }
}

// FsDir implementation would go here...
```

## Using Middleware Layers

Wrap any backend with middleware:

```rust
use anyfs_backend::{Fs, Layer};

// Assuming LoggingLayer is a middleware that logs operations
let fs = SimpleFs::new();
let fs = LoggingLayer::new("MyApp").layer(fs);

// Now all operations are logged
fs.write(Path::new("/hello.txt"), b"Hello!").unwrap();
```

## Next Steps

- [Trait Hierarchy](./trait-hierarchy.md) - Understand the layer system
- [Backend Tutorial](../tutorials/backend/README.md) - Complete backend implementation guide
- [Middleware Tutorial](../tutorials/middleware/README.md) - Create reusable layers
