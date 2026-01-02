# AGENTS.md - Instructions for AI Assistants

READ THIS FIRST before making any changes to this repository.

---

## Project Overview

`anyfs-backend` is the foundational crate for the AnyFS ecosystem. It defines:
- **Trait hierarchy** (`Fs`, `FsFull`, `FsFuse`, `FsPosix`)
- **Core types** (`Metadata`, `DirEntry`, `Permissions`, `FsError`, etc.)
- **Layer trait** for Tower-style middleware composition
- **Extension traits** (`FsExt`, `FsPath`)

This crate has **minimal dependencies** (`thiserror` required; `serde` optional).

> **Design Manual:** The authoritative design documentation is available at:
> - **Online:** <https://dk26.github.io/anyfs-design-manual/>
> - **Source:** Sibling repo `anyfs-design-manual`
>
> Reference it for architectural decisions, ADRs, trait specifications, and the big picture.

---

## Development Methodology: TDD (Test-Driven Development)

**We follow strict TDD: write tests first, then implement until green.**

### Tests Are Tasks

| Concept | Meaning |
|---------|---------|
| A failing test | A task to complete |
| A passing test | A completed task |
| All tests green | Feature/issue complete |
| Test count | Progress metric |

Each test represents a specific requirement or behavior. When you write a test, you define a task. When the test passes, you have completed that task.

**Track progress by counting green tests.**

### The TDD Cycle

```
1. RED:   Write a failing test (define a task)
2. GREEN: Write minimal code to pass (complete the task)
3. REFACTOR: Clean up while keeping tests green (polish)
```

### TDD Workflow for Each Issue

```
1. Read the issue and design manual section
2. Write test cases that define the API contract (tasks)
3. Run tests - they should FAIL (RED)
4. Implement the minimum code to pass tests
5. Run tests - they should PASS (GREEN)
6. Refactor for clarity, keep tests green
7. Add edge case tests, repeat cycle
```

---

## Implementation Order

Follow the GitHub issues in dependency order:

### Phase 1: Foundation
1. **#1 FsError** - Error types first (everything depends on this)
2. **#2 Core Types** - Metadata, DirEntry, Permissions, etc.
3. **#3 FsRead trait** - Read operations
4. **#4 FsWrite trait** - Write operations
5. **#5 FsDir trait** - Directory operations
6. **#6 Fs trait** - Combine FsRead + FsWrite + FsDir

### Phase 2: Extended Traits
7. **#7-#15** - FsLink, FsPermissions, FsSync, FsStats, etc.
8. **#16-#17** - FsFull, FsFuse composite traits

### Phase 3: Infrastructure
9. **#18 Layer trait** - Middleware composition
10. **#19 FsExt** - Convenience methods

---

## Test Categories

Each trait/type needs tests for:

| Category | What to Test |
|----------|--------------|
| **Type Tests** | Correct fields, derives, Send+Sync |
| **Error Tests** | All error variants, Display impl, From conversions |
| **Object Safety** | `dyn Trait` compiles and works |
| **Trait Bounds** | Supertraits are enforced |
| **Send+Sync** | Thread safety (compile-time check) |

---

## Code Style Requirements

### No Panic Policy
Always return `Result`, never `.unwrap()` in library code.

### Error Context
Always include path and operation in errors.

### Documentation
Every public item needs doc comments with `# Errors` and `# Examples` sections.

---

## Thread Safety (ADR-023)

All traits require `Send + Sync`. Methods take `&self` (not `&mut self`).

This means:
- Backends use interior mutability (`RwLock`, `Mutex`)
- Safe for concurrent access from multiple threads
- No exclusive references needed for operations

---

## Boxing Strategy (ADR-025)

| Path | Strategy |
|------|----------|
| Hot path (`read`, `write`, `metadata`) | Concrete types, generics |
| Cold path (`open_read`, `open_write`, `read_dir`) | `Box<dyn Read>`, etc. |
| Opt-in type erasure | `FileStorage::boxed()` |

**Do NOT** add boxing to hot path methods.

---

## When in Doubt

| Question | Answer |
|----------|--------|
| What goes in this crate? | Only traits and types, no implementations |
| Where are backends? | `anyfs` crate (not this one) |
| Should I add a dependency? | Probably not. Ask first. |
| Async support? | Sync only for now. Async-ready design. |
| How to test trait? | Mock struct in tests that implements it |
