// Copyright 2025 The Servo Project Developers.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! WebAssembly Text (WAT) to binary compilation

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use parking_lot::RwLock;

/// Error type for WASM compilation
#[derive(Debug)]
pub enum CompileError {
    ParseError(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::ParseError(msg) => write!(f, "WAT parse error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// Simple in-memory cache for compiled WASM
/// Maps hash(source_code) -> compiled binary as base64
fn get_cache() -> &'static RwLock<HashMap<u64, Vec<u8>>> {
    static CACHE: OnceLock<RwLock<HashMap<u64, Vec<u8>>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Compile WAT source code to WASM binary, then encode as base64 data URL
///
/// # Arguments
/// * `source` - The WAT (WebAssembly Text) source code
/// * `filename` - The name of the file (for error reporting)
///
/// # Returns
/// JavaScript code that loads the WASM module and exports its functions
pub fn compile_wat_to_js(source: &str, filename: &str) -> Result<String, CompileError> {
    eprintln!("💥 INSIDE wasm_compiler::compile_wat_to_js!");
    log::info!("WASM: Compiling {} ({} bytes)", filename, source.len());

    // Check cache first
    eprintln!("🔑 Calculating cache key...");
    let cache_key = calculate_hash(source);
    eprintln!("📦 Checking cache (key: {})...", cache_key);
    let wasm_binary = {
        // Check cache first - must drop read lock before attempting write
        let cached = {
            let cache = get_cache().read();
            cache.get(&cache_key).cloned()
        };

        if let Some(binary) = cached {
            eprintln!("✨ Cache HIT!");
            log::info!("WASM: Cache hit for {}", filename);
            binary
        } else {
            eprintln!("🆕 Cache MISS - compiling WAT...");
            // Compile WAT to WASM binary
            let binary = compile_wat_internal(source, filename)?;
            eprintln!("🎉 WAT compilation successful!");
            log::info!("WASM: Successfully compiled {} to {} bytes of WASM", filename, binary.len());

            // Store in cache (read lock is already dropped at this point)
            {
                let mut cache = get_cache().write();
                // Limit cache size to 100 entries (WASM modules can be large)
                if cache.len() > 100 {
                    cache.clear();
                }
                cache.insert(cache_key, binary.clone());
            }

            binary
        }
    };

    eprintln!("🎨 Generating JavaScript code from {} bytes of WASM binary...", wasm_binary.len());

    // Generate JavaScript byte array directly (no base64 encoding needed!)
    // This is the approach that works reliably in Servo
    eprintln!("📊 Starting byte array conversion...");
    let byte_array = wasm_binary
        .iter()
        .map(|b| format!("0x{:02X}", b))
        .collect::<Vec<_>>()
        .join(", ");

    eprintln!("✅ Byte array converted! Length: {} chars", byte_array.len());

    // Generate JavaScript that uses direct byte array
    // This avoids base64/atob issues and works perfectly in Servo
    eprintln!("🔨 Formatting JavaScript wrapper...");
    let js_code = format!(
        r#"
(function() {{
    try {{
        console.log('WASM: Starting module load');

        // WASM module as direct byte array (most reliable method)
        const wasmBytes = new Uint8Array([{}]);

        console.log('WASM: Instantiating module (' + wasmBytes.length + ' bytes)...');

        // Instantiate directly from byte array
        WebAssembly.instantiate(wasmBytes)
            .then(function(result) {{
                console.log('WASM: Module instantiated successfully');

                // Export all WASM functions to window
                if (result.instance && result.instance.exports) {{
                    for (const name in result.instance.exports) {{
                        const func = result.instance.exports[name];
                        if (typeof func === 'function') {{
                            // Export functions directly - SpiderMonkey now handles GC object property access natively
                            window[name] = func;
                            console.log('WASM: Exported function ' + name);
                        }}
                    }}

                    // Helper function to display GC struct contents
                    window.WasmGcStructDisplay = function(structObj, structName) {{
                        if (!structObj || typeof structObj !== 'object') {{
                            return String(structObj);
                        }}

                        structName = structName || 'box';
                        let fields = [];

                        // Try common field names
                        const commonFields = ['val', 'value', 'data', 'x', 'y', 'z', 'width', 'height'];
                        for (const fieldName of commonFields) {{
                            if (typeof WasmGcStructGet !== 'undefined') {{
                                try {{
                                    const fieldValue = WasmGcStructGet(structObj, fieldName);
                                    if (fieldValue !== undefined) {{
                                        fields.push(fieldName + '=' + fieldValue);
                                    }}
                                }} catch (e) {{
                                    // Field doesn't exist, skip
                                }}
                            }}
                        }}

                        if (fields.length > 0) {{
                            return structName + '{{' + fields.join(', ') + '}}';
                        }} else {{
                            return structName + '{{}}';
                        }}
                    }};

                    // Create GC struct field accessors
                    // For WASM GC structs, we need getter functions that call struct.get
                    // These are typically exported as 'get_field_X' functions by WASM
                    window.WasmGcStructGet = function(structObj, fieldIndex) {{
                        // Attempt to extract field value from GC struct
                        // Look for exported getter functions following common patterns
                        const getterName = 'get_' + fieldIndex;
                        if (window._wasmExports && window._wasmExports[getterName]) {{
                            try {{
                                return window._wasmExports[getterName](structObj);
                            }} catch (e) {{
                                console.warn('WasmGcStructGet: Getter', getterName, 'failed:', e);
                            }}
                        }}

                        // Fallback: try numeric field access patterns
                        const fieldGetter = 'struct_get_' + fieldIndex;
                        if (window._wasmExports && window._wasmExports[fieldGetter]) {{
                            try {{
                                return window._wasmExports[fieldGetter](structObj);
                            }} catch (e) {{
                                console.warn('WasmGcStructGet: Getter', fieldGetter, 'failed:', e);
                            }}
                        }}

                        // Try property access as last resort (for externref wrapping)
                        if (structObj && typeof structObj === 'object') {{
                            if (structObj[fieldIndex] !== undefined) {{
                                return structObj[fieldIndex];
                            }}
                            const fieldName = 'field' + fieldIndex;
                            if (structObj[fieldName] !== undefined) {{
                                return structObj[fieldName];
                            }}
                        }}

                        console.warn('WasmGcStructGet: Unable to access field', fieldIndex, 'on', structObj);
                        return undefined;
                    }};

                    // Helper to list available getter functions
                    window.WasmListGetters = function() {{
                        const getters = [];
                        for (const name in window._wasmExports) {{
                            if (name.startsWith('get_') || name.startsWith('struct_get_')) {{
                                getters.push(name);
                            }}
                        }}
                        return getters;
                    }};

                    console.log('WASM: GC struct accessors installed');
                    console.log('WASM: Available getters:', window.WasmListGetters());
                }}

                console.log('WASM module loaded successfully');
            }})
            .catch(function(e) {{
                console.error('WASM instantiation error:', e);
            }});

    }} catch (e) {{
        console.error('WASM error:', e);
    }}
}})();
"#,
        byte_array
    );

    eprintln!("🎉 JavaScript wrapper complete! Total size: {} chars", js_code.len());
    eprintln!("🚀 Returning compiled JS to caller...");

    Ok(js_code)
}

/// Internal compilation function using wat crate
fn compile_wat_internal(source: &str, filename: &str) -> Result<Vec<u8>, CompileError> {
    // Check if input is already binary WASM (starts with magic number \0asm)
    let source_bytes = source.as_bytes();
    let wasm_binary = if source_bytes.len() >= 4 && &source_bytes[0..4] == b"\0asm" {
        eprintln!("🔍 Detected binary WASM format (magic: \\0asm)");
        log::info!("WASM: Input is already binary WASM, using directly");
        // Already compiled, use the bytes
        source_bytes.to_vec()
    } else {
        // Otherwise, parse as WAT text format
        eprintln!("🔧 Calling wat::parse_str...");
        let result = wat::parse_str(source);
        match &result {
            Ok(bytes) => eprintln!("✅ wat::parse_str succeeded! {} bytes", bytes.len()),
            Err(e) => eprintln!("❌ wat::parse_str FAILED: {}", e),
        }
        result.map_err(|e| CompileError::ParseError(format!("in {}: {}", filename, e)))?
    };

    // Inject getter/setter functions for WASM GC structs
    inject_gc_accessors(&wasm_binary)
}

/// Inject getter/setter functions for WASM GC struct fields
fn inject_gc_accessors(wasm_binary: &[u8]) -> Result<Vec<u8>, CompileError> {
    eprintln!("🔬 Analyzing WASM for GC structs...");

    // Automatic getter/setter injection for WASM GC structs is complex and requires:
    // - Parsing type section to detect struct definitions
    // - Generating new function types for getters/setters
    // - Encoding struct.get/struct.set instructions
    // - Managing function/type indices correctly
    //
    // Given SpiderMonkey's architectural limitations (JIT blocks property access on
    // non-native objects) and the complexity of WASM binary manipulation, the pragmatic
    // approach is to require manual getter/setter exports in the WASM code.
    //
    // Example WAT with manual exports:
    //
    //   (module
    //     (type $box (struct (field $val (mut i32))))
    //     (func $makeBox (export "makeBox") (param i32) (result (ref $box))
    //       local.get 0
    //       struct.new $box
    //     )
    //     (func $get_val (export "get_val") (param (ref $box)) (result i32)
    //       local.get 0
    //       struct.get $box $val
    //     )
    //     (func $set_val (export "set_val") (param (ref $box)) (param i32)
    //       local.get 0
    //       local.get 1
    //       struct.set $box $val
    //     )
    //   )
    //
    // Then in JavaScript: get_val(box) instead of box.val

    eprintln!("ℹ️  Automatic accessor injection not implemented (requires complex WASM transformation)");
    eprintln!("💡 Please export getter/setter functions manually in your WASM code");
    eprintln!("   See test-wasm-gc-with-getters.html for a working example");

    Ok(wasm_binary.to_vec())
}

/// Calculate hash for caching
fn calculate_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Clear the compilation cache (useful for testing or memory management)
#[allow(dead_code)]
pub fn clear_cache() {
    get_cache().write().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_wasm() {
        let source = r#"
            (module
              (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add)
              (export "add" (func $add)))
        "#;

        let result = compile_wat_to_js(source, "test.wat");
        assert!(result.is_ok());

        let js = result.unwrap();
        assert!(js.contains("WebAssembly"));
        assert!(js.contains("data:application/wasm;base64,"));
    }

    #[test]
    fn test_caching() {
        clear_cache();

        let source = "(module)";

        // First compilation
        let result1 = compile_wat_to_js(source, "test.wat");
        assert!(result1.is_ok());

        // Second compilation (should hit cache)
        let result2 = compile_wat_to_js(source, "test.wat");
        assert!(result2.is_ok());

        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_invalid_wat() {
        let source = "(module (invalid syntax))";

        let result = compile_wat_to_js(source, "test.wat");
        assert!(result.is_err());
    }
}
