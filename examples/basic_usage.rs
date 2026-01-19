//! Basic usage of anyfs-backend traits.
//!
//! This example demonstrates the fundamental operations you can perform
//! with any filesystem backend that implements the `Fs` trait.
//!
//! Run with: `cargo run --example basic_usage`

use anyfs_backend::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::SystemTime;

// =============================================================================
// Step 1: Create a minimal filesystem implementation
// =============================================================================

/// A simple in-memory filesystem that implements `Fs` (the base trait).
///
/// This is the minimum viable implementation to use anyfs-backend.
/// It only implements FsRead, FsWrite, and FsDir.
struct SimpleFs {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
    dirs: RwLock<std::collections::HashSet<PathBuf>>,
}

impl SimpleFs {
    fn new() -> Self {
        let fs = Self {
            files: RwLock::new(HashMap::new()),
            dirs: RwLock::new(std::collections::HashSet::new()),
        };
        // Create root directory
        fs.dirs.write().unwrap().insert(PathBuf::from("/"));
        fs
    }
}

// Implement FsRead - reading files and metadata
impl FsRead for SimpleFs {
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
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|_| FsError::InvalidData {
            path: path.to_path_buf(),
            details: "not valid UTF-8".into(),
        })
    }

    fn read_range(&self, path: &Path, offset: u64, len: usize) -> Result<Vec<u8>, FsError> {
        let data = self.read(path)?;
        let start = offset as usize;
        if start >= data.len() {
            return Ok(Vec::new());
        }
        let end = (start + len).min(data.len());
        Ok(data[start..end].to_vec())
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
        let data = self.read(path)?;
        Ok(Box::new(std::io::Cursor::new(data)))
    }
}

// Implement FsWrite - writing and modifying files
impl FsWrite for SimpleFs {
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
        let mut files = self.files.write().unwrap();
        let data = files.remove(from).ok_or_else(|| FsError::NotFound {
            path: from.to_path_buf(),
        })?;
        files.insert(to.to_path_buf(), data);
        Ok(())
    }

    fn copy(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let data = self.read(from)?;
        self.write(to, &data)
    }

    fn truncate(&self, path: &Path, size: u64) -> Result<(), FsError> {
        let mut files = self.files.write().unwrap();
        let data = files.get_mut(path).ok_or_else(|| FsError::NotFound {
            path: path.to_path_buf(),
        })?;
        data.resize(size as usize, 0);
        Ok(())
    }

    fn open_write(&self, _path: &Path) -> Result<Box<dyn Write + Send>, FsError> {
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }
}

// Implement FsDir - directory operations
impl FsDir for SimpleFs {
    fn read_dir(&self, path: &Path) -> Result<ReadDirIter, FsError> {
        if !self.dirs.read().unwrap().contains(path) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        let mut entries = Vec::new();

        // Collect files in this directory
        for (file_path, data) in self.files.read().unwrap().iter() {
            if let Some(parent) = file_path.parent() {
                if parent == path {
                    if let Some(name) = file_path.file_name() {
                        entries.push(Ok(DirEntry {
                            name: name.to_string_lossy().into_owned(),
                            path: file_path.clone(),
                            file_type: FileType::File,
                            size: data.len() as u64,
                            inode: 0,
                        }));
                    }
                }
            }
        }

        // Collect subdirectories
        for dir_path in self.dirs.read().unwrap().iter() {
            if let Some(parent) = dir_path.parent() {
                if parent == path && dir_path != path {
                    if let Some(name) = dir_path.file_name() {
                        entries.push(Ok(DirEntry {
                            name: name.to_string_lossy().into_owned(),
                            path: dir_path.clone(),
                            file_type: FileType::Directory,
                            size: 0,
                            inode: 0,
                        }));
                    }
                }
            }
        }

        Ok(ReadDirIter::from_vec(entries))
    }

    fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        let mut dirs = self.dirs.write().unwrap();
        if dirs.contains(path) {
            return Err(FsError::AlreadyExists {
                path: path.to_path_buf(),
                operation: "create_dir",
            });
        }
        dirs.insert(path.to_path_buf());
        Ok(())
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
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
// Step 2: Use the filesystem
// =============================================================================

fn main() {
    println!("=== anyfs-backend Basic Usage Example ===\n");

    // Create our filesystem
    let fs = SimpleFs::new();

    // --- Writing files ---
    println!("1. Writing files...");
    fs.write(Path::new("/hello.txt"), b"Hello, World!").unwrap();
    fs.write(Path::new("/data.bin"), &[0x00, 0x01, 0x02, 0x03])
        .unwrap();
    println!("   Created /hello.txt and /data.bin");

    // --- Reading files ---
    println!("\n2. Reading files...");
    let text = fs.read_to_string(Path::new("/hello.txt")).unwrap();
    println!("   /hello.txt contains: {text}");

    let binary = fs.read(Path::new("/data.bin")).unwrap();
    println!("   /data.bin contains: {binary:?}");

    // --- Checking existence ---
    println!("\n3. Checking existence...");
    println!(
        "   /hello.txt exists: {}",
        fs.exists(Path::new("/hello.txt")).unwrap()
    );
    println!(
        "   /missing.txt exists: {}",
        fs.exists(Path::new("/missing.txt")).unwrap()
    );

    // --- Getting metadata ---
    println!("\n4. Getting metadata...");
    let meta = fs.metadata(Path::new("/hello.txt")).unwrap();
    println!(
        "   /hello.txt: type={:?}, size={}",
        meta.file_type, meta.size
    );

    // --- Directory operations ---
    println!("\n5. Directory operations...");
    fs.create_dir(Path::new("/subdir")).unwrap();
    fs.write(Path::new("/subdir/nested.txt"), b"Nested file")
        .unwrap();

    println!("   Created /subdir/nested.txt");
    println!("   Contents of /:");
    for entry in fs.read_dir(Path::new("/")).unwrap() {
        let entry = entry.unwrap();
        println!("     - {} ({:?})", entry.name, entry.file_type);
    }

    // --- Copy and rename ---
    println!("\n6. Copy and rename...");
    fs.copy(Path::new("/hello.txt"), Path::new("/hello_copy.txt"))
        .unwrap();
    fs.rename(Path::new("/hello_copy.txt"), Path::new("/greeting.txt"))
        .unwrap();
    println!("   Copied /hello.txt to /greeting.txt (via rename)");

    // --- Append ---
    println!("\n7. Appending to files...");
    fs.append(Path::new("/hello.txt"), b" Appended!").unwrap();
    let updated = fs.read_to_string(Path::new("/hello.txt")).unwrap();
    println!("   /hello.txt now contains: {updated}");

    // --- Error handling ---
    println!("\n8. Error handling...");
    match fs.read(Path::new("/nonexistent.txt")) {
        Ok(_) => println!("   Unexpected success"),
        Err(FsError::NotFound { path }) => {
            println!("   Correctly got NotFound for: {}", path.display());
        }
        Err(e) => println!("   Unexpected error: {e}"),
    }

    // --- Using the Fs trait bound ---
    println!("\n9. Using generic functions...");
    fn count_files<B: Fs>(fs: &B, dir: &Path) -> usize {
        fs.read_dir(dir)
            .map(|iter| iter.filter(|e| e.is_ok()).count())
            .unwrap_or(0)
    }
    println!("   Files in /: {}", count_files(&fs, Path::new("/")));

    println!("\n=== Example complete! ===");
}
