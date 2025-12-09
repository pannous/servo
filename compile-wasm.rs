/// Compile WAT to WASM binary using the wat crate
use std::fs;
use std::io::Write;

fn main() {
    let wat_source = fs::read_to_string("test-wasm-gc-simple.wat")
        .expect("Failed to read WAT file");

    match wat::parse_str(&wat_source) {
        Ok(wasm_bytes) => {
            fs::write("test-wasm-gc-simple.wasm", &wasm_bytes)
                .expect("Failed to write WASM file");
            println!("✓ Successfully compiled to test-wasm-gc-simple.wasm ({} bytes)", wasm_bytes.len());
        },
        Err(e) => {
            eprintln!("✗ Compilation error: {}", e);
            std::process::exit(1);
        }
    }
}
