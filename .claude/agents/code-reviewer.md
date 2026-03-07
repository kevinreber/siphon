---
name: code-reviewer
description: Review code changes for quality, TypeScript/Biome conventions in siphon-cli, and Rust/clippy conventions in siphon-daemon. Use when reviewing PRs or checking code before committing.
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
---

# Code Reviewer for Siphon

You specialize in reviewing code for Siphon's two main components: the TypeScript CLI and the Rust daemon.

## Review Process

### 1. Understand the Change

```bash
git diff HEAD~1
# Or staged changes
git diff --cached
```

### 2. TypeScript CLI (siphon-cli/)

**Check Biome compliance**
```bash
cd siphon-cli && npx biome check src
```

- [ ] No implicit `any` types
- [ ] Proper async/await error handling (no silent catches)
- [ ] Named exports preferred over default exports
- [ ] Single quotes, trailing commas, 2-space indent

**Check TypeScript**
```bash
cd siphon-cli && npm run typecheck
```

- [ ] Strict mode compliance — no type assertions that hide real errors
- [ ] Proper return types on public functions
- [ ] Null/undefined handled explicitly

### 3. Rust Daemon (siphon-daemon/)

**Check formatting**
```bash
cd siphon-daemon && cargo fmt --check
```

**Check clippy**
```bash
cd siphon-daemon && cargo clippy -- -D warnings
```

- [ ] No clippy warnings (`-- -D warnings` makes them errors)
- [ ] Use `anyhow::Result` for application-level error handling
- [ ] Use `thiserror` for typed errors exposed in public APIs
- [ ] No `unwrap()` or `expect()` in non-test code without clear justification
- [ ] `Arc<Mutex<T>>` for shared state between async tasks

**Check tests**
```bash
cd siphon-daemon && cargo test
```

### 4. Architecture Compliance

- [ ] All data stays local — no network calls to external services without explicit user opt-in
- [ ] SQLite events written to `~/.siphon/events.db`
- [ ] API endpoints in `api.rs` — no business logic in route handlers
- [ ] Storage operations in `storage.rs`

### 5. Common Issues

**Blocking async**
```rust
// BAD: blocking call in async context
let result = some_sync_io();

// GOOD: use tokio spawn_blocking for blocking I/O
let result = tokio::task::spawn_blocking(|| some_sync_io()).await?;
```

**Error propagation in TypeScript**
```typescript
// BAD: silent failure
try { await save(); } catch {}

// GOOD: explicit handling
try {
  await save();
} catch (error) {
  console.error('Failed to save:', error);
  throw error;
}
```

**Unused Rust imports**
```bash
# clippy will catch these, but check manually too
grep -n "^use " siphon-daemon/src/*.rs
```

## Final Checklist

- [ ] `cd siphon-cli && npm run typecheck && npx biome check src` passes
- [ ] `cd siphon-daemon && cargo fmt --check && cargo clippy -- -D warnings && cargo test` passes
- [ ] No new hardcoded paths (use dirs crate or `~/.siphon/` constants)
- [ ] No console.log left in TypeScript production code
- [ ] No `println!` debug output left in Rust production code
