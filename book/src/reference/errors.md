# Error Types

AnyFS defines a unified error type for all filesystem operations.

## FsError

The core error type used by all traits:

```rust
use anyfs_backend::FsError;
use std::path::Path;

fn handle_error(e: FsError) {
    match e {
        FsError::NotFound { path, operation } => {
            println!("{} not found during {}", path.display(), operation);
        }
        FsError::AlreadyExists { path, operation } => {
            println!("{} already exists during {}", path.display(), operation);
        }
        FsError::PermissionDenied { path, operation } => {
            println!("Permission denied: {} during {}", path.display(), operation);
        }
        FsError::Io { source, path, operation } => {
            println!("IO error on {}: {} during {}", 
                path.map(|p| p.display().to_string()).unwrap_or_default(),
                source, operation);
        }
        // ... handle other variants
    }
}
```

## Error Variants

| Variant             | When Used                       | Required Fields                          |
| ------------------- | ------------------------------- | ---------------------------------------- |
| `NotFound`          | File or directory doesn't exist | `path`, `operation`                      |
| `AlreadyExists`     | Creating something that exists  | `path`, `operation`                      |
| `PermissionDenied`  | Access not allowed              | `path`, `operation`                      |
| `IsDirectory`       | Expected file, got directory    | `path`, `operation`                      |
| `NotDirectory`      | Expected directory, got file    | `path`, `operation`                      |
| `DirectoryNotEmpty` | Removing non-empty directory    | `path`, `operation`                      |
| `InvalidPath`       | Malformed path                  | `path`, `operation`, `reason`            |
| `TooManySymlinks`   | Symlink loop detected           | `path`, `operation`                      |
| `ReadOnly`          | Write on read-only filesystem   | `path`, `operation`                      |
| `CrossDevice`       | Cross-filesystem operation      | `source`, `destination`, `operation`     |
| `Io`                | General I/O error               | `source`, `path` (optional), `operation` |
| `Other`             | Unclassified errors             | `message`, `operation`                   |

## Creating Errors

Use the constructor methods for clean error creation:

```rust
use anyfs_backend::FsError;
use std::path::Path;

// NotFound
let err = FsError::not_found(Path::new("/missing.txt"), "read");

// AlreadyExists
let err = FsError::already_exists(Path::new("/exists"), "create_dir");

// PermissionDenied
let err = FsError::permission_denied(Path::new("/secret"), "read");

// IsDirectory
let err = FsError::is_directory(Path::new("/folder"), "read");

// NotDirectory
let err = FsError::not_directory(Path::new("/file.txt"), "read_dir");

// DirectoryNotEmpty
let err = FsError::directory_not_empty(Path::new("/folder"), "remove_dir");

// InvalidPath
let err = FsError::invalid_path(
    Path::new("/bad\0path"),
    "contains null byte",
    "open"
);

// ReadOnly
let err = FsError::read_only(Path::new("/file.txt"), "write");

// IO error
let err = FsError::io(
    std::io::Error::new(std::io::ErrorKind::Other, "disk full"),
    Some(Path::new("/file.txt")),
    "write"
);
```

## Error Conversion

### From std::io::Error

```rust
use anyfs_backend::FsError;
use std::io;

fn from_io_error(e: io::Error, path: &Path, op: &str) -> FsError {
    FsError::io(e, Some(path), op)
}

// Or use From trait (without path context)
let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
let fs_err: FsError = io_err.into();
```

### To std::io::Error

```rust
use anyfs_backend::FsError;
use std::io;

let fs_err = FsError::not_found(Path::new("/missing"), "read");
let io_err: io::Error = fs_err.into();

assert_eq!(io_err.kind(), io::ErrorKind::NotFound);
```

## Error Display

All errors implement `Display` with helpful messages:

```rust
let err = FsError::not_found(Path::new("/file.txt"), "read");
println!("{}", err);
// Output: not found: /file.txt (during read)

let err = FsError::permission_denied(Path::new("/secret"), "open");
println!("{}", err);
// Output: permission denied: /secret (during open)
```

## Best Practices

### 1. Always Include Operation Context

```rust
// ✓ Good - includes operation
FsError::not_found(path, "read")

// ✗ Bad - no context
FsError::NotFound { path: path.into(), operation: String::new() }
```

### 2. Include Path When Available

```rust
// ✓ Good - includes path
FsError::io(e, Some(path), "write")

// ✗ Avoid - loses context
FsError::io(e, None, "write")
```

### 3. Use Specific Error Types

```rust
// ✓ Good - specific error
if path.exists() {
    return Err(FsError::already_exists(path, "create"));
}

// ✗ Avoid - generic error
return Err(FsError::other("file exists", "create"));
```

### 4. Pattern Match for Handling

```rust
match fs.read(path) {
    Ok(data) => process(data),
    Err(FsError::NotFound { .. }) => create_default(),
    Err(FsError::PermissionDenied { .. }) => request_access(),
    Err(e) => return Err(e),
}
```
