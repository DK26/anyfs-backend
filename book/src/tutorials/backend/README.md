# Implementing a Backend

This tutorial walks you through implementing a complete filesystem backend from scratch. By the end, you'll have a working in-memory filesystem that implements all traits up to `FsPosix`.

## What You'll Build

An in-memory filesystem (`TutorialFs`) that:
- Stores files and directories in memory
- Supports symlinks
- Tracks permissions and timestamps
- Provides inode-based access for FUSE
- Implements file handles and locking

## Prerequisites

- Basic Rust knowledge (structs, traits, `Result`)
- Understanding of filesystem concepts (files, directories, symlinks)

## Tutorial Structure

Each chapter introduces one or more traits:

1. **[Core Data Structures](./01-data-structures.md)** - Design the internal state
2. **[FsRead](./02-fs-read.md)** - Read files and metadata
3. **[FsWrite](./03-fs-write.md)** - Write and delete files
4. **[FsDir](./04-fs-dir.md)** - Directory operations
5. **[The Fs Trait](./05-fs-trait.md)** - Combining the basics
6. **[FsLink](./06-fs-link.md)** - Symlink support
7. **[FsFull](./07-fs-full.md)** - Permissions, sync, stats
8. **[FsInode](./08-fs-inode.md)** - FUSE inode operations
9. **[FsPosix](./09-fs-posix.md)** - Handles and locking

## Running the Examples

Each chapter has example code snippets. For runnable examples, check the `examples/` directory:

```bash
cargo run --example inmemory_fs
cargo run --example basic_usage
cargo run --example layer_middleware
```

## The End Result

After completing this tutorial, you'll understand:

- How each trait fits into the hierarchy
- What each method should do and return
- Error handling conventions
- Thread-safety requirements
- How blanket implementations work

Let's start with [Core Data Structures →](./01-data-structures.md)
