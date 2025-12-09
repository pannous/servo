# WASM GC Examples - Quick Start

## TL;DR - Just Run This

```bash
# Textual WAT format (recommended - most readable)
./mach run test-wat-gc-textual.html

# Or binary format
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

### Textual WAT Format (Recommended - Most Readable)
- **`test-wat-gc-textual.html`** ← Start here for GC features
  - Point struct with x/y coordinates
  - Distance calculation
  - Mutable fields with struct.get/struct.set
  - Uses `<script type="application/wasm">` with textual WAT

- **`test-wat-textual.html`** ← Basic calculator
  - add, subtract, multiply, divide
  - Simple i32 operations
  - Memory export example

### Binary Format Examples
- `test-wasm-gc-inline-binary.html` - 183-byte binary as hex
- `test-wasm-gc-load-binary.html` - Load binary via fetch()
- `test-wasm-gc-simple.html` - Inline WAT (older example)
- `test-wasm-gc-struct.html` - Complex struct (older example)

### Source Files
- `test-wasm-gc-simple.wat` - WASM source code
- `test-wasm-gc-simple.wasm` - Pre-compiled binary

## How It Works

### Textual WAT Format (Easiest!)

Just write WAT directly in a `<script>` tag:

```html
<script type="application/wasm">
  (module
    ;; Define struct type
    (type $Point (struct
      (field $x (mut f32))
      (field $y (mut f32))
    ))

    ;; Create instance
    (func $makePoint (export "makePoint")
      (param $x f32) (param $y f32)
      (result (ref $Point))
      local.get $x
      local.get $y
      struct.new $Point
    )

    ;; Read field
    (func $getX (export "getX")
      (param $p (ref $Point))
      (result f32)
      local.get $p
      struct.get $Point 0
    )
  )
</script>
```

JavaScript usage:
```javascript
const point = makePoint(3.0, 4.0);
const x = getX(point);  // 3.0
```

Servo automatically:
1. Detects `type="application/wasm"`
2. Compiles WAT to WASM binary using the `wat` crate
3. Generates JavaScript loader code
4. Exports all functions to `window`

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
