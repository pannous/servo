fn main() {
    let wat = std::fs::read_to_string("test-array-new-data.wat").unwrap();

    match wat::parse_str(&wat) {
        Ok(wasm) => {
            println!("✅ Compiled successfully! {} bytes", wasm.len());

            // Print first 100 bytes in hex
            println!("\nFirst bytes (hex):");
            for (i, chunk) in wasm.chunks(16).take(10).enumerate() {
                print!("{:04x}: ", i * 16);
                for byte in chunk {
                    print!("{:02x} ", byte);
                }
                println!();
            }

            // Look for the GC opcode 0xfb
            println!("\nSearching for GC opcodes (0xfb):");
            for (i, window) in wasm.windows(2).enumerate() {
                if window[0] == 0xfb {
                    println!("  Found 0xfb at offset {}: 0xfb 0x{:02x}", i, window[1]);
                }
            }
        }
        Err(e) => {
            println!("❌ Compilation failed: {}", e);
        }
    }
}
