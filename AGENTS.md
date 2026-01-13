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

## ⚠️ CRITICAL: Verify Against Design Manual

**Before implementing any trait, type, or API, ALWAYS verify against the design manual.**

### Verification Checklist

Before writing code, check the design manual for:

1. **Method signatures** - Exact parameter names, types, and return types
2. **Trait bounds** - Required supertraits (`Send + Sync`, etc.)
3. **Error variants** - Which `FsError` variants to return for each case
4. **Type definitions** - Field names, derives, and attributes
5. **Boxing strategy** - Which methods return `Box<dyn ...>` vs concrete types

### Key Design Manual Pages

| Topic           | URL                                                                            |
| --------------- | ------------------------------------------------------------------------------ |
| Design Overview | <https://dk26.github.io/anyfs-design-manual/architecture/design-overview.html> |
| Layered Traits  | <https://dk26.github.io/anyfs-design-manual/traits/layered-traits.html>        |
| ADRs            | <https://dk26.github.io/anyfs-design-manual/architecture/adrs.html>            |

### Why This Matters

- Trait signatures are **public API contracts** - changes are breaking
- The design manual is the **single source of truth**
- Early mistakes in trait design are **expensive to fix** later
- Tests should verify **design compliance**, not just functionality

### Workflow

```
1. Read the GitHub issue
2. Read the corresponding design manual section
3. Compare issue with design manual (design manual wins if conflict)
4. Write tests that match the design manual's API
5. Implement to pass tests
6. Verify final code matches design manual exactly
```

---

## Development Methodology: TDD (Test-Driven Development)

**We follow strict TDD: write tests first, then implement until green.**

### Tests Are Tasks

| Concept         | Meaning                |
| --------------- | ---------------------- |
| A failing test  | A task to complete     |
| A passing test  | A completed task       |
| All tests green | Feature/issue complete |
| Test count      | Progress metric        |

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

| Category          | What to Test                                       |
| ----------------- | -------------------------------------------------- |
| **Type Tests**    | Correct fields, derives, Send+Sync                 |
| **Error Tests**   | All error variants, Display impl, From conversions |
| **Object Safety** | `dyn Trait` compiles and works                     |
| **Trait Bounds**  | Supertraits are enforced                           |
| **Send+Sync**     | Thread safety (compile-time check)                 |

---

## ⛔ MANDATORY: All Tests and Doc Examples Must Compile and Run

**Every single test and doc example MUST compile and execute. No exceptions. No workarounds.**

### Absolutely Forbidden

| Forbidden                      | Why                                                        |
| ------------------------------ | ---------------------------------------------------------- |
| `#[ignore]`                    | Skips test execution                                       |
| `#[cfg(skip)]` or similar      | Conditionally disables tests                               |
| `skip` in any form             | Tests exist to run, not to be skipped                      |
| ` ```rust,ignore `             | Skips compilation entirely — NEVER use this                |
| ` ```no_run `                  | Code compiles but doesn't run — not allowed                |
| ` ```ignore `                  | Completely skips the code block                            |
| ` ```text ` for code examples  | Using text to avoid compilation — code must compile        |
| `compile_fail` without purpose | Only use when testing that code correctly fails to compile |
| Commenting out test code       | Dead code that pretends to be a test                       |
| Empty test functions           | A test must assert something                               |
| `todo!()` / `unimplemented!()` | Tests must be complete                                     |

### Allowed Uses

| Allowed                             | Why                                                  |
| ----------------------------------- | ---------------------------------------------------- |
| ` ```rust ` (default)               | Code compiles and runs — required for examples       |
| ` ```compile_fail `                 | Testing that invalid code fails to compile           |
| ` ```text ` for ASCII diagrams ONLY | Diagrams are not code — e.g., trait hierarchy arrows |

**Note:** ` ```text ` is ONLY acceptable for non-code content like ASCII art diagrams.
If it looks like Rust code, it MUST be ` ```rust ` and MUST compile.

### Writing Runnable Doc Examples for Trait-Only Crates

This crate defines **traits only**, with no concrete implementations. Doc examples must
still compile and run. Use inline mock implementations:

```rust
/// ```rust
/// use anyfs_backend::{FsRead, FsError, Metadata};
/// use std::path::Path;
/// use std::io::Read;
/// 
/// // Create a minimal mock that implements the trait
/// struct MockFs;
/// 
/// impl FsRead for MockFs {
///     fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
///         Ok(vec![1, 2, 3])
///     }
///     // ... implement other required methods
/// }
/// 
/// // Now use the mock in your example
/// let fs = MockFs;
/// let data = fs.read(Path::new("/test.txt")).unwrap();
/// assert_eq!(data, vec![1, 2, 3]);
/// ```
```

### Why This Is Non-Negotiable

1. **Tests are the specification** - A skipped test is a missing spec
2. **TDD requires running tests** - Can't be RED→GREEN if tests don't run
3. **CI will catch you** - All tests run in CI, no hiding
4. **Trust** - If you skip tests, how do we know the code works?
5. **Doc examples ARE tests** - They prove the API works as documented

### What To Do Instead

| Problem                          | Solution                                 |
| -------------------------------- | ---------------------------------------- |
| Test is hard to write            | Write it anyway, ask for help            |
| Feature not implemented yet      | Write the test, let it fail (RED phase)  |
| Test requires complex setup      | Create test utilities, mock structs      |
| Test is flaky                    | Fix the flakiness, don't skip            |
| "I'll fix it later"              | No. Fix it now or don't commit           |
| Trait has no implementation      | Create a minimal mock in the doc example |
| Doc example references `backend` | Define the backend struct in the example |

### Verification

Before any PR:

```bash
cargo test              # All tests must pass
cargo doc --document-private-items  # All doc tests must compile
```

If a test doesn't pass, **fix the code or fix the test**. Never disable the test.

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

| Path                                              | Strategy                 |
| ------------------------------------------------- | ------------------------ |
| Hot path (`read`, `write`, `metadata`)            | Concrete types, generics |
| Cold path (`open_read`, `open_write`, `read_dir`) | `Box<dyn Read>`, etc.    |
| Opt-in type erasure                               | `FileStorage::boxed()`   |

**Do NOT** add boxing to hot path methods.

---

## When in Doubt

| Question                   | Answer                                    |
| -------------------------- | ----------------------------------------- |
| What goes in this crate?   | Only traits and types, no implementations |
| Where are backends?        | `anyfs` crate (not this one)              |
| Should I add a dependency? | Probably not. Ask first.                  |
| Async support?             | Sync only for now. Async-ready design.    |
| How to test trait?         | Mock struct in tests that implements it   |
