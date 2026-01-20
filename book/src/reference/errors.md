# Error Types

AnyFS defines a unified error type for all filesystem operations.

## FsError

The core error type used by all traits:

```rust
use anyfs_backend::FsError;
use std::path::PathBuf;

fn handle_error(e: FsError) {
    match e {
        FsError::NotFound { path } => {
            println!("{} not found", path.display());
        }
        FsError::AlreadyExists { path, operation } => {
            println!("{} already exists during {}", path.display(), operation);
        }
        FsError::PermissionDenied { path, operation } => {
            println!("Permission denied: {} during {}", path.display(), operation);
        }
        FsError::Io { source, path, operation } => {
            println!("IO error on {}: {} during {}", 
                path.display(), source, operation);
        }
        // ... handle other variants
        _ => println!("Other error: {}", e),
    }
}
```

## Error Variants

| Variant             | When Used                       | Fields                        |
| ------------------- | ------------------------------- | ----------------------------- |
| `NotFound`          | File or directory doesn't exist | `path`                        |
| `AlreadyExists`     | Creating something that exists  | `path`, `operation`           |
| `PermissionDenied`  | Access not allowed              | `path`, `operation`           |
| `NotAFile`          | Expected file, got directory    | `path`                        |
| `NotADirectory`     | Expected directory, got file    | `path`                        |
| `DirectoryNotEmpty` | Removing non-empty directory    | `path`                        |
| `InvalidPath`       | Malformed path                  | `path`, `reason`              |
| `SymlinkLoop`       | Symlink loop detected           | `path`                        |
| `ReadOnly`          | Write on read-only filesystem   | `operation`                   |
| `ThreatDetected`    | Security threat detected        | `path`, `reason`              |
| `InvalidHandle`     | Invalid file handle             | `handle`                      |
| `Io`                | General I/O error               | `operation`, `path`, `source` |

## Creating Errors

Create errors directly with struct syntax:

```rust
use anyfs_backend::FsError;
use std::path::PathBuf;

// NotFound
let err = FsError::NotFound { 
    path: PathBuf::from("/missing.txt") 
};

// AlreadyExists
let err = FsError::AlreadyExists { 
    path: PathBuf::from("/exists"),
    operation: "create_dir"
};

// PermissionDenied
let err = FsError::PermissionDenied { 
    path: PathBuf::from("/secret"),
    operation: "read"
};

// NotAFile (for directory when file expected)
let err = FsError::NotAFile { 
    path: PathBuf::from("/folder") 
};

// NotADirectory
let err = FsError::NotADirectory { 
    path: PathBuf::from("/file.txt") 
};

// DirectoryNotEmpty
let err = FsError::DirectoryNotEmpty { 
    path: PathBuf::from("/folder") 
};

// InvalidPath
let err = FsError::InvalidPath { 
    path: PathBuf::from("/bad\0path"),
    reason: "contains null byte".to_string()
};

// ReadOnly
let err = FsError::ReadOnly { 
    operation: "write" 
};
```

## Error Conversion

### From std::io::Error

`FsError` implements `From<std::io::Error>`. Common error kinds are mapped to specific variants:

```rust
use anyfs_backend::FsError;
use std::io;

// From trait maps common kinds automatically
let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
let fs_err: FsError = io_err.into();
// Results in FsError::NotFound { path: PathBuf::new() }

let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
let fs_err: FsError = io_err.into();
// Results in FsError::PermissionDenied { path: PathBuf::new(), operation: "io" }

// Other errors become FsError::Io
let io_err = io::Error::new(io::ErrorKind::Other, "disk full");
let fs_err: FsError = io_err.into();
// Results in FsError::Io { operation: "io", path: PathBuf::new(), source }
```

> **Note:** The `From` conversion uses empty paths. For better context, construct the error directly with the actual path.

## Error Display

All errors implement `Display` with helpful messages:

```rust
use anyfs_backend::FsError;
use std::path::PathBuf;

let err = FsError::NotFound { path: PathBuf::from("/file.txt") };
println!("{}", err);
// Output: not found: /file.txt

let err = FsError::PermissionDenied { 
    path: PathBuf::from("/secret"),
    operation: "open"
};
println!("{}", err);
// Output: open: permission denied: /secret
```

## Best Practices

### 1. Include Path Context

```rust
// ✓ Good - includes path
FsError::NotFound { path: path.to_path_buf() }

// ✗ Avoid - empty path loses context
FsError::NotFound { path: PathBuf::new() }
```

### 2. Use Specific Error Types

```rust
// ✓ Good - specific error
if is_directory {
    return Err(FsError::NotAFile { path: path.to_path_buf() });
}

// ✗ Avoid - generic error when specific exists
return Err(FsError::Io { ... });
```

### 3. Pattern Match for Handling

```rust
match fs.read(path) {
    Ok(data) => process(data),
    Err(FsError::NotFound { .. }) => create_default(),
    Err(FsError::PermissionDenied { .. }) => request_access(),
    Err(e) => return Err(e),
}
```
