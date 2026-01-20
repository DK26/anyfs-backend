# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-pre.2] - 2026-01-20

### Added
- **Complete trait reference tables** in README.md and crate documentation for quick lookup of all 12 component traits and 4 composite traits
- **Comprehensive mdbook documentation** with organized book structure and enhanced HTML output
- **GitHub Pages integration** - User Guide now available at https://dk26.github.io/anyfs-backend/

### Changed
- **Documentation structure** - Reorganized book source (`/book/src`) with build output to `/docs` for GitHub Pages
- **User Guide links** - Added prominent links to both User Guide and Design Manual in README.md
- **mdbook configuration** - Enhanced with smart punctuation, code playground, fold sections, and search
- **Edit URL template** - Updated to point to correct source location

## [0.1.0-pre.1] - 2026-01-19

Initial pre-release of the AnyFS backend trait library.

### Added

#### Core Traits
- **`FsRead`** - Read operations: `read`, `read_to_string`, `read_range`, `exists`, `metadata`, `open_read`
- **`FsWrite`** - Write operations: `write`, `append`, `create_dir`, `create_dir_all`, `remove_file`, `remove_dir`, `remove_dir_all`, `copy`, `rename`, `open_write`
- **`FsDir`** - Directory operations: `read_dir` returning `ReadDirIter`
- **`Fs`** - Composite trait combining `FsRead + FsWrite + FsDir`

#### Extended Traits
- **`FsLink`** - Symlink and hard link operations: `symlink`, `hard_link`, `read_link`, `symlink_metadata`
- **`FsPermissions`** - Permission management: `set_permissions`, `set_owner`
- **`FsSync`** - Durability guarantees: `sync`, `sync_path`
- **`FsStats`** - Filesystem statistics: `statfs` returning `StatFs`
- **`FsFull`** - Composite trait combining `Fs + FsLink + FsPermissions + FsSync + FsStats`

#### FUSE Traits
- **`FsInode`** - Inode-based operations: `path_to_inode`, `inode_to_path`, `lookup`, `metadata_by_inode`
- **`FsFuse`** - Composite trait combining `FsFull + FsInode`

#### POSIX Traits
- **`FsHandles`** - Handle-based I/O: `open`, `close`, `read_at`, `write_at`
- **`FsLock`** - File locking: `lock`, `try_lock`, `unlock`
- **`FsXattr`** - Extended attributes: `get_xattr`, `set_xattr`, `remove_xattr`, `list_xattr`
- **`FsPosix`** - Composite trait combining `FsFuse + FsHandles + FsLock + FsXattr`

#### Path Resolution
- **`FsPath`** - Path canonicalization: `canonicalize`, `soft_canonicalize`
- **`PathResolver`** - Boxable path resolution trait for dynamic dispatch

#### Middleware Support
- **`Layer`** - Tower-style middleware composition for filesystem operations
- **`LayerExt`** - Extension trait for ergonomic layer chaining

#### Extension Traits
- **`FsExt`** - Convenience methods: `is_file`, `is_dir`, `is_symlink`, `file_size`
- **`FsExtJson`** (feature: `serde`) - JSON serialization: `read_json`, `write_json`

#### Marker Traits
- **`SelfResolving`** - Marker for backends that handle their own path resolution

#### Core Types
- **`FsError`** - Comprehensive error type with 15 variants and full context
- **`Metadata`** - File metadata: size, file type, timestamps, permissions, inode
- **`DirEntry`** - Directory entry with name, file type, inode, and metadata
- **`Permissions`** - Unix-style permission bits with readonly/executable helpers
- **`FileType`** - Enum: `File`, `Directory`, `Symlink`
- **`StatFs`** - Filesystem statistics: total/free/available space, inodes
- **`Handle`** - Opaque file handle for POSIX operations
- **`OpenFlags`** - File open flags: read, write, create, truncate, append, exclusive
- **`LockType`** - Lock types: `Shared`, `Exclusive`
- **`ReadDirIter`** - Boxed iterator for directory entries with `collect_all()` helper
- **`ROOT_INODE`** - Constant for root directory inode (1)

#### Documentation
- Comprehensive rustdoc with examples for all public items
- mdBook documentation site
- `LLM_CONTEXT.md` - Context7-style reference for AI agents
- `AGENTS.md` - Development guidelines for AI assistants

#### CI/CD
- GitHub Actions workflows: CI, security audit, release, semver checks
- Cross-platform testing: Linux, Windows, macOS
- MSRV verification (Rust 1.68)
- WASM build verification
- Feature matrix testing
- Code policy enforcement (no `#[allow(...)]`, no `ignore`/`no_run` in doctests)

### Features
- **`serde`** - Optional serialization support for all public types + `FsExtJson` trait

### Notes
- All traits require `Send + Sync` for thread safety
- All methods take `&self` (not `&mut self`) for concurrent access
- Minimal dependencies: only `thiserror` required, `serde` optional
- MSRV: Rust 1.68
