# Core Types

This reference documents the core types used throughout AnyFS.

## Metadata

File and directory metadata:

```rust
use anyfs_backend::Metadata;

let meta = fs.metadata(path)?;

// Check type
if meta.is_file() {
    println!("Size: {} bytes", meta.len());
} else if meta.is_dir() {
    println!("Directory");
} else if meta.is_symlink() {
    println!("Symlink");
}

// Timestamps (optional - may be None for some backends)
if let Some(created) = meta.created() {
    println!("Created: {:?}", created);
}
if let Some(modified) = meta.modified() {
    println!("Modified: {:?}", modified);
}
if let Some(accessed) = meta.accessed() {
    println!("Accessed: {:?}", accessed);
}

// Permissions (if available)
if let Some(perms) = meta.permissions() {
    println!("Readonly: {}", perms.readonly());
}
```

### Metadata Fields

| Field         | Type                  | Description                       |
| ------------- | --------------------- | --------------------------------- |
| `file_type`   | `FileType`            | File, directory, or symlink       |
| `len`         | `u64`                 | Size in bytes (0 for directories) |
| `created`     | `Option<SystemTime>`  | Creation time                     |
| `modified`    | `Option<SystemTime>`  | Last modification                 |
| `accessed`    | `Option<SystemTime>`  | Last access                       |
| `permissions` | `Option<Permissions>` | Permission info                   |

### Creating Metadata

For backend implementations:

```rust
use anyfs_backend::{Metadata, FileType, Permissions};
use std::time::SystemTime;

// File metadata
let meta = Metadata::file(1024)
    .with_created(SystemTime::now())
    .with_modified(SystemTime::now())
    .with_permissions(Permissions::readonly(false));

// Directory metadata
let meta = Metadata::dir()
    .with_modified(SystemTime::now());

// Symlink metadata
let meta = Metadata::symlink()
    .with_modified(SystemTime::now());
```

## FileType

Enumeration of filesystem entry types:

```rust
use anyfs_backend::FileType;

let ft = metadata.file_type();

match ft {
    FileType::File => println!("Regular file"),
    FileType::Dir => println!("Directory"),
    FileType::Symlink => println!("Symbolic link"),
}

// Convenience methods
assert!(FileType::File.is_file());
assert!(FileType::Dir.is_dir());
assert!(FileType::Symlink.is_symlink());
```

## DirEntry

Entry returned when reading directories:

```rust
use anyfs_backend::DirEntry;

for entry in fs.read_dir(path)? {
    let entry = entry?;
    
    // Name of the entry (not full path)
    println!("Name: {}", entry.name());
    
    // Full path
    println!("Path: {}", entry.path().display());
    
    // Type (if available without extra syscall)
    if let Some(ft) = entry.file_type() {
        println!("Type: {:?}", ft);
    }
    
    // Full metadata (may require extra syscall)
    let meta = entry.metadata()?;
    println!("Size: {}", meta.len());
}
```

### DirEntry Fields

| Method        | Return Type                 | Description           |
| ------------- | --------------------------- | --------------------- |
| `name()`      | `&str`                      | Entry name (not path) |
| `path()`      | `&Path`                     | Full path             |
| `file_type()` | `Option<FileType>`          | Type if known cheaply |
| `metadata()`  | `Result<Metadata, FsError>` | Full metadata         |

## Permissions

File permission information:

```rust
use anyfs_backend::Permissions;

// Create permissions
let perms = Permissions::readonly(false);  // read-write
let perms = Permissions::readonly(true);   // read-only

// Check permissions
if perms.readonly() {
    println!("File is read-only");
}

// POSIX mode (if supported)
#[cfg(unix)]
{
    let perms = Permissions::from_mode(0o755);
    println!("Mode: {:o}", perms.mode());
}
```

### Extended Permissions (Unix)

For backends that support POSIX permissions:

```rust
use anyfs_backend::Permissions;

// From mode bits
let perms = Permissions::from_mode(0o644);

// Check mode
let mode = perms.mode();  // 0o644

// Permission bits
let owner_read = (mode & 0o400) != 0;
let owner_write = (mode & 0o200) != 0;
let owner_exec = (mode & 0o100) != 0;
```

## OpenOptions

Options for opening files:

```rust
use anyfs_backend::OpenOptions;

// Read only (default)
let opts = OpenOptions::new().read(true);

// Write, create if missing
let opts = OpenOptions::new()
    .write(true)
    .create(true);

// Append mode
let opts = OpenOptions::new()
    .append(true)
    .create(true);

// Create new (fail if exists)
let opts = OpenOptions::new()
    .write(true)
    .create_new(true);

// Truncate existing
let opts = OpenOptions::new()
    .write(true)
    .truncate(true);
```

### OpenOptions Fields

| Method             | Default | Description             |
| ------------------ | ------- | ----------------------- |
| `read(bool)`       | `true`  | Open for reading        |
| `write(bool)`      | `false` | Open for writing        |
| `append(bool)`     | `false` | Append to end           |
| `create(bool)`     | `false` | Create if missing       |
| `create_new(bool)` | `false` | Create, fail if exists  |
| `truncate(bool)`   | `false` | Truncate to zero length |

## SeekFrom

Position for seeking within files:

```rust
use std::io::SeekFrom;

// From start of file
let pos = SeekFrom::Start(100);

// From end of file (negative offset)
let pos = SeekFrom::End(-50);

// From current position
let pos = SeekFrom::Current(25);
```

Used with file handles:

```rust
use std::io::{Read, Seek, SeekFrom};

let mut handle = fs.open_read(path)?;

// Jump to offset 100
handle.seek(SeekFrom::Start(100))?;

// Read from there
let mut buf = [0u8; 50];
handle.read(&mut buf)?;
```

## FileTimes

For setting file timestamps:

```rust
use anyfs_backend::FileTimes;
use std::time::SystemTime;

let times = FileTimes::new()
    .set_accessed(SystemTime::now())
    .set_modified(SystemTime::now());

fs.set_times(path, times)?;
```

## FsStats

Filesystem statistics (capacity, usage):

```rust
use anyfs_backend::FsStats;

let stats: FsStats = fs.stats()?;

println!("Total: {} bytes", stats.total_bytes);
println!("Free: {} bytes", stats.free_bytes);
println!("Available: {} bytes", stats.available_bytes);
println!("Used: {}%", 
    (stats.total_bytes - stats.available_bytes) * 100 / stats.total_bytes
);
```

### FsStats Fields

| Field             | Type          | Description           |
| ----------------- | ------------- | --------------------- |
| `total_bytes`     | `u64`         | Total capacity        |
| `free_bytes`      | `u64`         | Free space            |
| `available_bytes` | `u64`         | Available to non-root |
| `total_inodes`    | `Option<u64>` | Total inodes (Unix)   |
| `free_inodes`     | `Option<u64>` | Free inodes (Unix)    |

## InodeId

Unique identifier for files (used by FsInode trait):

```rust
use anyfs_backend::InodeId;

let inode = fs.inode(path)?;
println!("Inode: {}", inode);

// Compare inodes to check if same file
let inode1 = fs.inode(path1)?;
let inode2 = fs.inode(path2)?;
if inode1 == inode2 {
    println!("Same file (hard links)");
}
```

## Summary

| Type          | Purpose                                  |
| ------------- | ---------------------------------------- |
| `Metadata`    | File/directory attributes                |
| `FileType`    | File, Dir, or Symlink                    |
| `DirEntry`    | Directory listing entry                  |
| `Permissions` | Access permissions                       |
| `OpenOptions` | File open configuration                  |
| `FileTimes`   | Timestamp modification                   |
| `FsStats`     | Filesystem capacity                      |
| `InodeId`     | Unique file identifier                   |
| `FsError`     | Error handling (see [Errors](errors.md)) |
