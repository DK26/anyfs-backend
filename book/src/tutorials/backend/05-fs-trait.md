# The Fs Trait

`Fs` is the first composite trait. It combines `FsRead`, `FsWrite`, and `FsDir` into a single bound.

## Automatic Implementation

Here's the magic: **you don't implement `Fs` directly**. It's automatically provided:

```rust
// In anyfs-backend (simplified)
pub trait Fs: FsRead + FsWrite + FsDir + Send + Sync {}

// Blanket implementation
impl<T> Fs for T where T: FsRead + FsWrite + FsDir + Send + Sync {}
```

If your type implements `FsRead + FsWrite + FsDir` and is `Send + Sync`, it automatically implements `Fs`.

## Verify Your Implementation

After implementing the three component traits, verify `Fs` works:

```rust
// Compile-time verification
fn use_fs<B: Fs>(_: &B) {}

fn main() {
    let fs = TutorialFs::new();
    use_fs(&fs);  // ✅ Compiles! TutorialFs implements Fs
}
```

You can also verify `Send + Sync` at compile time:

```rust
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TutorialFs>();
};
```

## Using the Fs Bound

Now you can write generic functions that work with any filesystem:

```rust
use anyfs_backend::{Fs, FsError};
use std::path::Path;

/// Copy a file from src to dst.
fn copy_file<B: Fs>(fs: &B, src: &Path, dst: &Path) -> Result<(), FsError> {
    let content = fs.read(src)?;
    fs.write(dst, &content)
}

/// Count files in a directory (non-recursive).
fn count_files<B: Fs>(fs: &B, dir: &Path) -> Result<usize, FsError> {
    let mut count = 0;
    for entry in fs.read_dir(dir)? {
        let entry = entry?;
        if entry.file_type == FileType::File {
            count += 1;
        }
    }
    Ok(count)
}

/// Check if a path is a file.
fn is_file<B: Fs>(fs: &B, path: &Path) -> bool {
    fs.metadata(path)
        .map(|m| m.file_type == FileType::File)
        .unwrap_or(false)
}
```

## What Fs Provides

At this point, your backend supports:

| Operation          | Method                              |
| ------------------ | ----------------------------------- |
| Read file contents | `read()`                            |
| Get metadata       | `metadata()`                        |
| Check existence    | `exists()`                          |
| Write file         | `write()`                           |
| Delete file        | `remove_file()`                     |
| List directory     | `read_dir()`                        |
| Create directory   | `create_dir()` / `create_dir_all()` |
| Remove directory   | `remove_dir()` / `remove_dir_all()` |
| Rename/move        | `rename()`                          |

This is sufficient for many use cases!

## What's Missing?

`Fs` doesn't include:
- Symlinks (`FsLink`)
- Permissions (`FsPermissions`)
- Sync/flush (`FsSync`)
- Disk stats (`FsStats`)
- Inode operations (`FsInode`)
- File handles (`FsHandles`)
- Locking (`FsLock`)

The remaining tutorials add these features.

## Integration Test

Here's a complete test exercising `Fs`:

```rust
#[test]
fn test_fs_workflow() {
    let fs = TutorialFs::new();

    // Create a project structure
    fs.create_dir_all(Path::new("/project/src")).unwrap();
    
    // Write some files
    fs.write(Path::new("/project/README.md"), b"# My Project").unwrap();
    fs.write(Path::new("/project/src/main.rs"), b"fn main() {}").unwrap();
    
    // Verify structure
    assert!(fs.exists(Path::new("/project")));
    assert!(fs.exists(Path::new("/project/src")));
    assert!(fs.exists(Path::new("/project/README.md")));
    
    // Read back
    let readme = fs.read(Path::new("/project/README.md")).unwrap();
    assert_eq!(readme, b"# My Project");
    
    // List directory
    let entries: Vec<_> = fs.read_dir(Path::new("/project"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 2);  // README.md and src/
    
    // Rename
    fs.rename(
        Path::new("/project/README.md"),
        Path::new("/project/README.txt"),
    ).unwrap();
    assert!(!fs.exists(Path::new("/project/README.md")));
    assert!(fs.exists(Path::new("/project/README.txt")));
    
    // Clean up
    fs.remove_dir_all(Path::new("/project")).unwrap();
    assert!(!fs.exists(Path::new("/project")));
}
```

## Summary

- `Fs = FsRead + FsWrite + FsDir + Send + Sync`
- **Automatically implemented** via blanket impl
- Provides basic file operations
- Sufficient for simple use cases

Next: [FsLink: Symlinks →](./06-fs-link.md)
