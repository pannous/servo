# WASM GC Examples - Quick Start

## TL;DR - Just Run This

```bash
./mach run test-wasm-gc-inline-binary.html
```

This demonstrates WASM GC structs with field accessors, working directly from `file://` URLs.

## What You'll See

The test creates WASM GC structs (boxes containing i32 values) and:
- ✓ Creates a box with value 42
- ✓ Reads the value (struct.get) → 42
- ✓ Modifies the value (struct.set) → 100
- ✓ Creates multiple isolated boxes
- ✓ Verifies struct isolation (modifying one doesn't affect others)

## Why This Matters

WASM GC structs are **opaque objects** in JavaScript - you can't directly access their fields. You need exported getter/setter functions that use `struct.get`/`struct.set` instructions.

## The Files

### Ready to Run (no server needed)
- **`test-wasm-gc-inline-binary.html`** ← Start here
  - Embeds 183-byte WASM binary as hex string
  - Works with `file://` URLs
  - No network/fetch required

### Other Examples
- `test-wasm-gc-simple.html` - Inline WAT source (auto-compiled)
- `test-wasm-gc-struct.html` - Complex struct with multiple fields

### Source Files
- `test-wasm-gc-simple.wat` - WASM source code
- `test-wasm-gc-simple.wasm` - Pre-compiled binary

## How It Works

```wat
;; Define struct type
(type $box (struct (field $val (mut i32))))

;; Create instance
(func $makeBox (param $v i32) (result (ref $box))
  local.get $v
  struct.new $box
)

;; Read field (this is KEY!)
(func $getValue (param $b (ref $box)) (result i32)
  local.get $b
  struct.get $box 0
)
```

JavaScript usage:
```javascript
const box = makeBox(42);
const value = getValue(box);  // 42
setValue(box, 100);
```

## Browser Enhancements

The WASM compiler (in `components/script/wasm_compiler.rs`) adds:
- `window._wasmExports` - All exported functions
- `WasmGcStructGet(obj, field)` - Helper to access struct fields
- `WasmListGetters()` - List available getters

## Full Documentation

See `WASM-GC-GUIDE.md` for complete details.

## Compiling Your Own

```bash
# Run the Rust test to compile
cargo test --package script --test wasm_gc_compile

# Or use wat2wasm (if you have it)
wat2wasm your-file.wat -o your-file.wasm --enable-gc
```
