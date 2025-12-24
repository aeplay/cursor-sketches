# Sled + WASM + IndexedDB Analysis

## Summary

| Question | Answer |
|----------|--------|
| Can sled compile to WASM? | ✅ **Yes** (sled 0.34.x compiles; 1.0-alpha requires Rust edition 2024) |
| Can IndexedDB be used as a backend? | ❌ **No** (not supported - sled is tightly coupled to filesystem I/O) |
| Would it be faster than plain IndexedDB? | ⚠️ **No practical benefit** (see details below) |
| Bundle size | ~357 KB (sled) vs ~290 KB (idb wrapper) |

---

## Detailed Analysis

### 1. Can Sled Be Used Through WASM?

**Yes, but with significant limitations.**

Sled 0.34.7 (the stable version) successfully compiles to `wasm32-unknown-unknown`:

```bash
cargo build --target wasm32-unknown-unknown --release
# ✅ Compiles successfully
# Output: 357 KB WASM file (365,004 bytes)
```

However, **sled 1.0-alpha.124** (the latest development version) currently **cannot compile** because it depends on `inline-array` which requires Rust edition 2024 (not yet stable).

### 2. Storage Backend Limitations

**Sled cannot use IndexedDB as a backend.** Here's why:

#### Sled's Architecture

Sled is a **log-structured storage engine** that relies on:
- Direct filesystem access (`std::fs`)
- Memory-mapped files (mmap)
- File locking (`fs2` crate)
- fsync for durability guarantees

These are fundamental to sled's design and cannot be abstracted away. The crate has no pluggable storage interface.

#### WASM Storage Options

In WASM, sled can only operate in **temporary/in-memory mode**:

```rust
// This is the ONLY way sled works in WASM
let db = sled::Config::new().temporary(true).open()?;
```

This creates an ephemeral in-memory database that:
- ✅ Works for the session
- ❌ Cannot persist data after page refresh
- ❌ Cannot use IndexedDB, localStorage, or any browser storage

### 3. Would Sled + IndexedDB Be Faster Than Plain IndexedDB?

**No, this approach would not provide performance benefits.**

Even if you could somehow adapt sled to use IndexedDB (which you can't without forking and rewriting the entire storage layer), you would hit these issues:

| Factor | Impact |
|--------|--------|
| **Double serialization** | Sled serializes data → then serialize again for IndexedDB |
| **Async mismatch** | Sled expects synchronous I/O; IndexedDB is purely async |
| **Transaction model conflict** | Sled's ACID transactions don't map to IndexedDB transactions |
| **No mmap equivalent** | Sled's zero-copy reads rely on mmap, impossible in browser |
| **Write amplification** | Sled's log-structured writes would multiply IndexedDB operations |

**IndexedDB's performance characteristics:**
- Already has B-tree indices internally
- Optimized for browser's storage layer
- Has its own transaction isolation
- Directly accessible from JavaScript/WASM

Adding sled as a layer on top would only add overhead.

### 4. Bundle Size Comparison

| Library | WASM Size (release, optimized) |
|---------|-------------------------------|
| **sled 0.34** | 357 KB (365,004 bytes) |
| **idb wrapper** | 290 KB (296,825 bytes) |
| **Difference** | sled is ~23% larger |

Note: These are pre-wasm-opt sizes with `opt-level = "z"`, LTO, and strip enabled. With `wasm-opt`, both could shrink by 10-30%.

---

## Recommended Alternatives

If you need a KV store in WASM that persists to IndexedDB:

### 1. **Direct IndexedDB** (via `idb` crate)
- Smallest bundle size
- Full browser API access
- Best performance for browser use

```toml
[dependencies]
idb = "0.6"
```

### 2. **GlueSQL with IndexedDB backend**
- SQL-like interface
- Built-in IndexedDB storage support

```toml
[dependencies]
gluesql = { version = "0.18", features = ["idb-storage"] }
```

### 3. **localForage bindings**
- Cross-browser abstraction
- Automatic fallback (IndexedDB → WebSQL → localStorage)

### 4. **sql.js** (SQLite compiled to WASM)
- Full SQL database
- Can persist to IndexedDB with extra wrapper
- Well-tested, widely used

---

## Conclusion

**Sled compiles to WASM but cannot use IndexedDB as a backend.** The architecture is fundamentally incompatible:

1. Sled requires synchronous filesystem access (mmap, fsync, file locks)
2. IndexedDB is purely async with a different transaction model
3. There's no pluggable storage interface in sled

For browser-based KV storage, **use IndexedDB directly** (via the `idb` crate) or **GlueSQL with its IndexedDB backend**. These are purpose-built for the browser environment and will outperform any attempt to layer a native database on top of browser storage APIs.

---

## Test Project Structure

The analysis was performed by creating test projects:

```
/workspace/
├── sled-wasm-analysis/          # Sled WASM compilation test
│   ├── Cargo.toml
│   └── src/lib.rs
├── indexeddb-comparison/        # IndexedDB size comparison
│   ├── Cargo.toml
│   └── src/lib.rs
└── SLED_WASM_INDEXEDDB_ANALYSIS.md
```
