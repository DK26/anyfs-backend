# Contributing to anyfs-backend

Thanks for your interest in contributing! 🦀

## Quick Start

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/anyfs-backend.git`
3. Test locally:
   - Linux/macOS/WSL: `bash ci-local.sh`
   - Windows PowerShell: `.\ci-local.ps1`
4. Submit a pull request

## How to Contribute

- 🐛 Bug reports: [Open an issue](https://github.com/DK26/anyfs-backend/issues) with reproduction steps
- 💡 Features: Discuss in an issue before implementing (see [Feature Suggestions](#feature-suggestions) below)
- 📝 Docs: Fix typos, add examples, improve clarity
- 🔧 Code: Bug fixes and improvements welcome

## Issue Template

Copy, paste, and fill what you need. Delete unused lines:

```markdown
What: [Brief description]

Why: [Problem or motivation]

Idea: [Your proposed solution]

Example: [Code or use case]

Benefits: [Who gains, what improves]

Links: [Related docs/issues]
```

## Feature Suggestions

Before suggesting features, **have your LLM agent read our [`AGENTS.md`](./AGENTS.md) and [`LLM_CONTEXT.md`](./LLM_CONTEXT.md) files** and ask it:

1. Does my suggested feature align with the project's design philosophy?
2. Why might this feature not already be implemented?
3. How does this fit within existing API patterns?

**Important:** This crate defines **traits only** — no implementations. Backend implementations belong in the `anyfs` crate. If you're proposing a new filesystem feature, consider whether it should be:
- A new trait method (this crate)
- A new backend implementation (`anyfs` crate)
- A middleware layer (either crate)

**Timeline expectations:**
- **Within design philosophy:** May be added in minor releases
- **Outside design philosophy:** Requires major version (potentially far future unless critical)

We encourage **all** suggestions! The distinction just helps set implementation expectations.

## Development

**Project Philosophy:**
- Traits define contracts — implementations live elsewhere
- All traits require `Send + Sync` for thread safety
- Methods take `&self` (not `&mut self`) for concurrent access
- Minimal dependencies (`thiserror` only; `serde` optional)
- Follow the [Design Manual](https://dk26.github.io/anyfs-design-manual/)

**Before implementing:**
1. Read the relevant [Design Manual](https://dk26.github.io/anyfs-design-manual/) section
2. Verify method signatures match the specification exactly
3. Write tests first (TDD methodology — see `AGENTS.md`)

## Testing

Run the CI script locally:

```bash
# Linux/macOS/WSL
bash ci-local.sh

# Windows PowerShell  
.\ci-local.ps1
```

If it passes, your code is ready.

### What CI Checks

| Check    | Purpose                                               |
| -------- | ----------------------------------------------------- |
| Format   | `cargo fmt --check`                                   |
| Clippy   | Lints with `-D warnings` (all features + no features) |
| Tests    | Unit, integration, and doc tests                      |
| Docs     | Documentation builds without warnings                 |
| Features | All feature combinations compile                      |
| MSRV     | Compiles on Rust 1.68                                 |
| Policy   | No `#[allow(...)]`, no `ignore`/`no_run` in doctests  |
| Safety   | No undocumented `unsafe` code                         |

## Code Style

- **No panics:** Always return `Result`, never `.unwrap()` in library code
- **Error context:** Include path and operation in all errors
- **Documentation:** Every public item needs doc comments with `# Errors` and `# Examples`
- **Tests compile and run:** No `#[ignore]`, no `no_run` in doctests (see `AGENTS.md`)

## License

By contributing, you agree that your contributions will be licensed under MIT OR Apache-2.0.

## Getting Help

- **Issues:** Bug reports and feature requests
- **Design Manual:** [dk26.github.io/anyfs-design-manual](https://dk26.github.io/anyfs-design-manual/)
- **Email:** [dikaveman@gmail.com](mailto:dikaveman@gmail.com)

Every contribution matters! 🚀
