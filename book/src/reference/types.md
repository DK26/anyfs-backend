# Core Types

This reference documents the core types used throughout AnyFS.

## Metadata

File and directory metadata:

```rust
use anyfs_backend::Metadata;

let meta = fs.metadata(path)?;

// Check type
if meta.is_file() {
    println!("Size: {} bytes", meta.size);
} else if meta.is_dir() {
    println!("Directory");
} else if meta.is_symlink() {
    println!("Symlink");
}

// Timestamps (concrete values)
println!("Created: {:?}", meta.created);
println!("Modified: {:?}", meta.modified);
println!("Accessed: {:?}", meta.accessed);

// Permissions
println!("Readonly: {}", meta.permissions.readonly());
```

### Metadata Fields

| Field         | Type          | Description                       |
| ------------- | ------------- | --------------------------------- |
| `file_type`   | `FileType`    | File, Directory, or Symlink       |
| `size`        | `u64`         | Size in bytes (0 for directories) |
| `permissions` | `Permissions` | Permission bits                   |
| `created`     | `SystemTime`  | Creation time                     |
| `modified`    | `SystemTime`  | Last modification                 |
| `accessed`    | `SystemTime`  | Last access                       |
| `inode`       | `u64`         | Inode number                      |
| `nlink`       | `u64`         | Hard link count                   |

### Creating Metadata

For backend implementations:

```rust
use anyfs_backend::{Metadata, FileType, Permissions};
use std::time::SystemTime;

// Use Default and modify fields
let mut meta = Metadata::default();
meta.file_type = FileType::File;
meta.size = 1024;
meta.permissions = Permissions::from_mode(0o644);
meta.modified = SystemTime::now();
```

## FileType

Enumeration of filesystem entry types:

```rust
use anyfs_backend::FileType;

let ft = meta.file_type;

match ft {
    FileType::File => println!("Regular file"),
    FileType::Directory => println!("Directory"),
    FileType::Symlink => println!("Symbolic link"),
}

// Metadata convenience methods
assert!(meta.is_file());
assert!(meta.is_dir());
assert!(meta.is_symlink());
```

## DirEntry

Entry returned when reading directories:

```rust
use anyfs_backend::DirEntry;

for entry in fs.read_dir(path)? {
    let entry = entry?;
    
    // Name of the entry (filename only)
    println!("Name: {}", entry.name);
    
    // Full path
    println!("Path: {}", entry.path.display());
    
    // Type
    println!("Type: {:?}", entry.file_type);
    
    // Size
    println!("Size: {} bytes", entry.size);
}
```

### DirEntry Fields

| Field       | Type       | Description              |
| ----------- | ---------- | ------------------------ |
| `name`      | `String`   | Entry name (not path)    |
| `path`      | `PathBuf`  | Full path                |
| `file_type` | `FileType` | File, Directory, Symlink |
| `size`      | `u64`      | Size in bytes            |
| `inode`     | `u64`      | Inode number             |

## Permissions

Unix-style permission bits:

```rust
use anyfs_backend::Permissions;

// Create from mode bits
let perms = Permissions::from_mode(0o644);  // rw-r--r--
let perms = Permissions::from_mode(0o755);  // rwxr-xr-x

// Check if readonly
if perms.readonly() {
    println!("File is read-only (no write bits set)");
}

// Get the mode value
println!("Mode: {:o}", perms.mode());

// Default permissions
let file_perms = Permissions::default_file();  // 0o644
let dir_perms = Permissions::default_dir();    // 0o755
```

### Permissions Methods

| Method           | Return Type   | Description                     |
| ---------------- | ------------- | ------------------------------- |
| `from_mode(u32)` | `Permissions` | Create from Unix mode bits      |
| `mode()`         | `u32`         | Get the raw mode value          |
| `readonly()`     | `bool`        | True if no write bits set       |
| `default_file()` | `Permissions` | Default file perms (0o644)      |
| `default_dir()`  | `Permissions` | Default directory perms (0o755) |

## OpenFlags

Flags for opening files with handle-based APIs (FsHandles):

```rust
use anyfs_backend::OpenFlags;

// Use predefined constants
let flags = OpenFlags::READ;       // Read only
let flags = OpenFlags::WRITE;      // Write + create + truncate
let flags = OpenFlags::READ_WRITE; // Read and write
let flags = OpenFlags::APPEND;     // Append mode

// Or construct manually
let flags = OpenFlags {
    read: true,
    write: true,
    create: true,
    truncate: false,
    append: false,
};
```

### OpenFlags Fields

| Field      | Type   | Description             |
| ---------- | ------ | ----------------------- |
| `read`     | `bool` | Open for reading        |
| `write`    | `bool` | Open for writing        |
| `create`   | `bool` | Create if missing       |
| `truncate` | `bool` | Truncate to zero length |
| `append`   | `bool` | Append to end           |

## StatFs

Filesystem statistics (from FsStats trait):

```rust
use anyfs_backend::StatFs;

let stats: StatFs = fs.statfs()?;

println!("Total: {} bytes", stats.total_bytes);
println!("Available: {} bytes", stats.available_bytes);
println!("Used: {} bytes", stats.used_bytes);
```

### StatFs Fields

| Field              | Type  | Description                            |
| ------------------ | ----- | -------------------------------------- |
| `total_bytes`      | `u64` | Total capacity (0 = unlimited)         |
| `used_bytes`       | `u64` | Currently used bytes                   |
| `available_bytes`  | `u64` | Available bytes for use                |
| `total_inodes`     | `u64` | Total number of inodes (0 = unlimited) |
| `used_inodes`      | `u64` | Number of used inodes                  |
| `available_inodes` | `u64` | Number of available inodes             |
| `block_size`       | `u64` | Block size in bytes                    |
| `max_name_len`     | `u64` | Maximum filename length                |

## Handle

Opaque file handle for POSIX-style APIs (FsHandles):

```rust
use anyfs_backend::Handle;

// Open a file
let handle = fs.open(path, OpenFlags::READ)?;

// Read/write with handle
let mut buf = [0u8; 1024];
let bytes_read = fs.read_at(handle, &mut buf, 0)?;

// Close when done
fs.close(handle)?;
```

## Summary

| Type          | Purpose                                  |
| ------------- | ---------------------------------------- |
| `Metadata`    | File/directory attributes                |
| `FileType`    | File, Directory, or Symlink              |
| `DirEntry`    | Directory listing entry                  |
| `Permissions` | Unix permission bits                     |
| `OpenFlags`   | File open configuration (for FsHandles)  |
| `StatFs`      | Filesystem capacity (from FsStats)       |
| `Handle`      | Opaque file handle (for FsHandles)       |
| `FsError`     | Error handling (see [Errors](errors.md)) |
