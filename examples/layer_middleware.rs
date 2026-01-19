//! Using the Layer trait for middleware composition.
//!
//! This example demonstrates how to use the Tower-style Layer pattern
//! to add cross-cutting functionality (logging, caching, metrics) to
//! any filesystem backend.
//!
//! Run with: `cargo run --example layer_middleware`

use anyfs_backend::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::SystemTime;

// =============================================================================
// Layer 1: Logging Layer
// =============================================================================

/// A layer that logs all filesystem operations.
struct LoggingLayer;

/// The logging middleware that wraps any Fs backend.
struct LoggingFs<B> {
    inner: B,
    prefix: String,
}

impl<B: Fs> Layer<B> for LoggingLayer {
    type Backend = LoggingFs<B>;

    fn layer(self, inner: B) -> Self::Backend {
        LoggingFs {
            inner,
            prefix: "[LOG]".to_string(),
        }
    }
}

impl<B: FsRead> FsRead for LoggingFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        println!("{} read({})", self.prefix, path.display());
        let result = self.inner.read(path);
        println!(
            "{} read({}) -> {:?}",
            self.prefix,
            path.display(),
            result.as_ref().map(|d| format!("{} bytes", d.len()))
        );
        result
    }

    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        println!("{} read_to_string({})", self.prefix, path.display());
        self.inner.read_to_string(path)
    }

    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        println!(
            "{} read_range({}, {}, {})",
            self.prefix,
            path.display(),
            offset,
            len
        );
        self.inner.read_range(path, offset, len)
    }

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        println!("{} exists({})", self.prefix, path.display());
        self.inner.exists(path)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        println!("{} metadata({})", self.prefix, path.display());
        self.inner.metadata(path)
    }

    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
        println!("{} open_read({})", self.prefix, path.display());
        self.inner.open_read(path)
    }
}

impl<B: FsWrite> FsWrite for LoggingFs<B> {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        println!(
            "{} write({}, {} bytes)",
            self.prefix,
            path.display(),
            data.len()
        );
        self.inner.write(path, data)
    }

    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        println!(
            "{} append({}, {} bytes)",
            self.prefix,
            path.display(),
            data.len()
        );
        self.inner.append(path, data)
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        println!("{} remove_file({})", self.prefix, path.display());
        self.inner.remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        println!(
            "{} rename({} -> {})",
            self.prefix,
            from.display(),
            to.display()
        );
        self.inner.rename(from, to)
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        println!(
            "{} copy({} -> {})",
            self.prefix,
            from.display(),
            to.display()
        );
        self.inner.copy(from, to)
    }

    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> {
        println!("{} truncate({}, {})", self.prefix, path.display(), size);
        self.inner.truncate(path, size)
    }

    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        println!("{} open_write({})", self.prefix, path.display());
        self.inner.open_write(path)
    }
}

impl<B: FsDir> FsDir for LoggingFs<B> {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        println!("{} read_dir({})", self.prefix, path.display());
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        println!("{} create_dir({})", self.prefix, path.display());
        self.inner.create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        println!("{} create_dir_all({})", self.prefix, path.display());
        self.inner.create_dir_all(path)
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        println!("{} remove_dir({})", self.prefix, path.display());
        self.inner.remove_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        println!("{} remove_dir_all({})", self.prefix, path.display());
        self.inner.remove_dir_all(path)
    }
}

// =============================================================================
// Layer 2: Metrics Layer
// =============================================================================

/// A layer that tracks operation counts.
struct MetricsLayer;

/// The metrics middleware with atomic counters.
struct MetricsFs<B> {
    inner: B,
    reads: AtomicUsize,
    writes: AtomicUsize,
    deletes: AtomicUsize,
}

impl<B> MetricsFs<B> {
    fn stats(&self) -> (usize, usize, usize) {
        (
            self.reads.load(Ordering::Relaxed),
            self.writes.load(Ordering::Relaxed),
            self.deletes.load(Ordering::Relaxed),
        )
    }
}

impl<B: Fs> Layer<B> for MetricsLayer {
    type Backend = MetricsFs<B>;

    fn layer(self, inner: B) -> Self::Backend {
        MetricsFs {
            inner,
            reads: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            deletes: AtomicUsize::new(0),
        }
    }
}

impl<B: FsRead> FsRead for MetricsFs<B> {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.read(path)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.read_to_string(path)
    }

    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.read_range(path, offset, len)
    }

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        self.inner.exists(path)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        self.inner.metadata(path)
    }

    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.open_read(path)
    }
}

impl<B: FsWrite> FsWrite for MetricsFs<B> {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.inner.write(path, data)
    }

    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.inner.append(path, data)
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.deletes.fetch_add(1, Ordering::Relaxed);
        self.inner.remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        self.inner.rename(from, to)
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.inner.copy(from, to)
    }

    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.inner.truncate(path, size)
    }

    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.inner.open_write(path)
    }
}

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
        self.deletes.fetch_add(1, Ordering::Relaxed);
        self.inner.remove_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        self.deletes.fetch_add(1, Ordering::Relaxed);
        self.inner.remove_dir_all(path)
    }
}

// =============================================================================
// Layer 3: Read-Only Layer
// =============================================================================

/// A layer that makes the filesystem read-only.
struct ReadOnlyLayer;

/// The read-only middleware that rejects all writes.
struct ReadOnlyFs<B> {
    inner: B,
}

impl<B: Fs> Layer<B> for ReadOnlyLayer {
    type Backend = ReadOnlyFs<B>;

    fn layer(self, inner: B) -> Self::Backend {
        ReadOnlyFs { inner }
    }
}

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

    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        self.inner.exists(path)
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        self.inner.metadata(path)
    }

    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
        self.inner.open_read(path)
    }
}

impl<B: FsRead> FsWrite for ReadOnlyFs<B> {
    fn write(&self, path: &Path, _data: &[u8]) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "write (read-only filesystem)",
        })
    }

    fn append(&self, path: &Path, _data: &[u8]) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "append (read-only filesystem)",
        })
    }

    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "remove_file (read-only filesystem)",
        })
    }

    fn rename(&self, from: &Path, _to: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: from.to_path_buf(),
            operation: "rename (read-only filesystem)",
        })
    }

    fn copy(&self, from: &Path, _to: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: from.to_path_buf(),
            operation: "copy (read-only filesystem)",
        })
    }

    fn truncate(&self, path: &Path, _size: u64) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "truncate (read-only filesystem)",
        })
    }

    fn open_write(&self, path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "open_write (read-only filesystem)",
        })
    }
}

impl<B: FsDir> FsDir for ReadOnlyFs<B> {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        self.inner.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "create_dir (read-only filesystem)",
        })
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "create_dir_all (read-only filesystem)",
        })
    }

    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "remove_dir (read-only filesystem)",
        })
    }

    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        Err(FsError::PermissionDenied {
            path: path.to_path_buf(),
            operation: "remove_dir_all (read-only filesystem)",
        })
    }
}

// =============================================================================
// Simple In-Memory Backend (same as other examples)
// =============================================================================

struct MemoryFs {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
    dirs: RwLock<std::collections::HashSet<PathBuf>>,
}

impl MemoryFs {
    fn new() -> Self {
        let fs = Self {
            files: RwLock::new(HashMap::new()),
            dirs: RwLock::new(std::collections::HashSet::new()),
        };
        fs.dirs.write().unwrap().insert(PathBuf::from("/"));
        fs
    }
}

impl FsRead for MemoryFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        self.files
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })
    }
    fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        String::from_utf8(self.read(path)?).map_err(|_| FsError::InvalidData {
            path: path.to_path_buf(),
            details: "not UTF-8".into(),
        })
    }
    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        let data = self.read(path)?;
        let start = offset as usize;
        Ok(if start >= data.len() {
            vec![]
        } else {
            data[start..(start + len).min(data.len())].to_vec()
        })
    }
    fn exists(&self, path: &Path) -> Result<bool, FsError> {
        Ok(self.files.read().unwrap().contains_key(path)
            || self.dirs.read().unwrap().contains(path))
    }
    fn metadata(&self, path: &Path) -> Result<Metadata, FsError> {
        if self.dirs.read().unwrap().contains(path) {
            Ok(Metadata {
                file_type: FileType::Directory,
                size: 0,
                permissions: Permissions::default_dir(),
                created: SystemTime::UNIX_EPOCH,
                modified: SystemTime::UNIX_EPOCH,
                accessed: SystemTime::UNIX_EPOCH,
                inode: 0,
                nlink: 1,
            })
        } else if let Some(data) = self.files.read().unwrap().get(path) {
            Ok(Metadata {
                file_type: FileType::File,
                size: data.len() as u64,
                permissions: Permissions::default_file(),
                created: SystemTime::UNIX_EPOCH,
                modified: SystemTime::UNIX_EPOCH,
                accessed: SystemTime::UNIX_EPOCH,
                inode: 0,
                nlink: 1,
            })
        } else {
            Err(FsError::NotFound {
                path: path.to_path_buf(),
            })
        }
    }
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>, FsError> {
        Ok(Box::new(std::io::Cursor::new(self.read(path)?)))
    }
}

impl FsWrite for MemoryFs {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .insert(path.to_path_buf(), data.to_vec());
        Ok(())
    }
    fn append(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .entry(path.to_path_buf())
            .or_default()
            .extend_from_slice(data);
        Ok(())
    }
    fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .remove(path)
            .map(|_| ())
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let data = self
            .files
            .write()
            .unwrap()
            .remove(from)
            .ok_or_else(|| FsError::NotFound {
                path: from.to_path_buf(),
            })?;
        self.files.write().unwrap().insert(to.to_path_buf(), data);
        Ok(())
    }
    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let data = self.read(from)?;
        self.write(to, &data)
    }
    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> {
        self.files
            .write()
            .unwrap()
            .get_mut(path)
            .ok_or_else(|| FsError::NotFound {
                path: path.to_path_buf(),
            })?
            .resize(size as usize, 0);
        Ok(())
    }
    fn open_write(&self, _path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }
}

impl FsDir for MemoryFs {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        if !self.dirs.read().unwrap().contains(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        let mut entries = Vec::new();
        for (fp, data) in self.files.read().unwrap().iter() {
            if fp.parent() == Some(path) {
                if let Some(name) = fp.file_name() {
                    entries.push(Ok(DirEntry {
                        name: name.to_string_lossy().into(),
                        path: fp.clone(),
                        file_type: FileType::File,
                        size: data.len() as u64,
                        inode: 0,
                    }));
                }
            }
        }
        for dp in self.dirs.read().unwrap().iter() {
            if dp.parent() == Some(path) && dp != path {
                if let Some(name) = dp.file_name() {
                    entries.push(Ok(DirEntry {
                        name: name.to_string_lossy().into(),
                        path: dp.clone(),
                        file_type: FileType::Directory,
                        size: 0,
                        inode: 0,
                    }));
                }
            }
        }
        Ok(ReadDirIter::from_vec(entries))
    }
    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        if self.dirs.read().unwrap().contains(path) {
            return Err(FsError::AlreadyExists {
                path: path.to_path_buf(),
                operation: "create_dir",
            });
        }
        self.dirs.write().unwrap().insert(path.to_path_buf());
        Ok(())
    }
    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        let mut current = PathBuf::new();
        for c in path.components() {
            current.push(c);
            self.dirs.write().unwrap().insert(current.clone());
        }
        Ok(())
    }
    fn remove_dir(&self, path: &Path) -> Result<(), FsError> {
        if !self.dirs.write().unwrap().remove(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }
        Ok(())
    }
    fn remove_dir_all(&self, path: &Path) -> Result<(), FsError> {
        self.dirs.write().unwrap().remove(path);
        self.files
            .write()
            .unwrap()
            .retain(|p, _| !p.starts_with(path));
        Ok(())
    }
}

// =============================================================================
// Main: Demonstrate layer composition
// =============================================================================

fn main() {
    println!("=== Layer Middleware Example ===\n");

    // Create base filesystem
    let base = MemoryFs::new();

    // Write some initial data
    base.write(Path::new("/config.txt"), b"initial config")
        .unwrap();
    base.create_dir(Path::new("/data")).unwrap();
    base.write(Path::new("/data/file1.txt"), b"file 1 content")
        .unwrap();

    // ===========================================
    // Example 1: Logging layer
    // ===========================================
    println!("--- Example 1: Logging Layer ---\n");

    // Using LayerExt for fluent API
    let logged_fs = base.layer(LoggingLayer);

    println!("Reading a file through logging layer:");
    let _ = logged_fs.read(Path::new("/config.txt"));
    println!();

    // Get base back (we'll use a new one for other examples)
    let base = MemoryFs::new();
    base.write(Path::new("/config.txt"), b"initial config")
        .unwrap();
    base.create_dir(Path::new("/data")).unwrap();
    base.write(Path::new("/data/file1.txt"), b"file 1 content")
        .unwrap();
    base.write(Path::new("/data/file2.txt"), b"file 2 content")
        .unwrap();

    // ===========================================
    // Example 2: Metrics layer
    // ===========================================
    println!("--- Example 2: Metrics Layer ---\n");

    let metrics_layer = MetricsLayer;
    let metrics_fs = metrics_layer.layer(MemoryFs::new());

    // Setup
    metrics_fs.write(Path::new("/a.txt"), b"a").unwrap();
    metrics_fs.write(Path::new("/b.txt"), b"b").unwrap();
    metrics_fs.write(Path::new("/c.txt"), b"c").unwrap();
    let _ = metrics_fs.read(Path::new("/a.txt"));
    let _ = metrics_fs.read(Path::new("/b.txt"));
    metrics_fs.remove_file(Path::new("/c.txt")).unwrap();

    let (reads, writes, deletes) = metrics_fs.stats();
    println!("Metrics: reads={reads}, writes={writes}, deletes={deletes}\n");

    // ===========================================
    // Example 3: Read-only layer
    // ===========================================
    println!("--- Example 3: Read-Only Layer ---\n");

    let readonly_fs = base.layer(ReadOnlyLayer);

    // Reading works
    println!("Reading through read-only layer:");
    match readonly_fs.read(Path::new("/config.txt")) {
        Ok(data) => println!("  Success: {} bytes", data.len()),
        Err(e) => println!("  Error: {e}"),
    }

    // Writing fails
    println!("Writing through read-only layer:");
    match readonly_fs.write(Path::new("/new.txt"), b"test") {
        Ok(()) => println!("  Success (unexpected!)"),
        Err(e) => println!("  Error: {e}"),
    }
    println!();

    // ===========================================
    // Example 4: Composing multiple layers
    // ===========================================
    println!("--- Example 4: Composing Layers ---\n");

    // Create a stack: MemoryFs -> Metrics -> Logging
    // Operations flow: Logging -> Metrics -> MemoryFs
    let base = MemoryFs::new();
    let with_metrics = MetricsLayer.layer(base);
    let with_logging = LoggingLayer.layer(with_metrics);

    println!("Writing through Logging -> Metrics -> Memory:");
    with_logging
        .write(Path::new("/test.txt"), b"hello")
        .unwrap();
    println!();

    println!("Reading through Logging -> Metrics -> Memory:");
    let _ = with_logging.read(Path::new("/test.txt"));
    println!();

    // Access inner metrics
    let (reads, writes, _) = with_logging.inner.stats();
    println!("Inner metrics: reads={reads}, writes={writes}\n");

    // ===========================================
    // Example 5: Using with trait objects
    // ===========================================
    println!("--- Example 5: Layers with Trait Objects ---\n");

    fn use_any_fs(fs: &dyn Fs) {
        let _ = fs.exists(Path::new("/"));
        println!("  Used filesystem through trait object");
    }

    let base = MemoryFs::new();
    let layered = base.layer(LoggingLayer);

    println!("Passing layered FS as &dyn Fs:");
    use_any_fs(&layered);

    println!("\n=== Layer examples complete! ===");
}
