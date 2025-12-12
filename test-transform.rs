fn transform_string_literal_in_line(line: &str) -> String {
    // Find struct.new position first
    if let Some(struct_new_pos) = line.find("struct.new") {
        // Only look for string literals AFTER struct.new
        let after_struct_new = &line[struct_new_pos..];

        if let Some(start_quote) = after_struct_new.find('"') {
            let absolute_start_quote = struct_new_pos + start_quote;

            if let Some(end_quote) = after_struct_new[start_quote + 1..].find('"') {
                let literal_start = absolute_start_quote + 1;
                let literal_end = absolute_start_quote + 1 + end_quote;
                let string_content = &line[literal_start..literal_end];

                // Convert string to UTF-8 bytes
                let utf8_bytes: Vec<String> = string_content
                    .as_bytes()
                    .iter()
                    .map(|b| format!("(i32.const {})", b))
                    .collect();

                let array_init = format!(
                    "(array.new_fixed $string {} {})",
                    utf8_bytes.len(),
                    utf8_bytes.join(" ")
                );

                // Replace the string literal with array initialization
                let before = &line[..absolute_start_quote];
                let after = &line[literal_end + 1..];
                return format!("{}{}{}", before, array_init, after);
            }
        }
    }

    line.to_string()
}

fn transform_string_types(source: &str) -> String {
    let mut result = String::new();
    let mut in_module = false;
    let mut string_type_added = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Detect module start to inject string type definition
        if trimmed.starts_with("(module") {
            result.push_str(line);
            result.push('\n');
            in_module = true;
            continue;
        }

        // Add string type definition right after module start, before any other content
        if in_module && !string_type_added && !trimmed.is_empty() && !trimmed.starts_with(";") {
            // Insert string type before any module content
            result.push_str("  ;; String type: array of i8 (UTF-8)\n");
            result.push_str("  (type $string (array (mut i8)))\n\n");
            string_type_added = true;
        }

        // Transform string literals in struct.new
        if trimmed.contains("struct.new") && trimmed.contains("\"") {
            result.push_str(&transform_string_literal_in_line(line));
            result.push('\n');
            continue;
        }

        // Replace 'string' type references with '(ref null $string)'
        let transformed = if line.contains("string") && !line.contains("$string") {
            // Replace type references: (mut string) -> (mut (ref null $string))
            let mut new_line = line.to_string();

            // Handle field definitions: (field $name (mut string))
            new_line = new_line.replace("(mut string)", "(mut (ref null $string))");

            // Handle param/result: (param string) or (result string)
            new_line = new_line.replace("(param string)", "(param (ref null $string))");
            new_line = new_line.replace("(result string)", "(result (ref null $string))");

            new_line
        } else {
            line.to_string()
        };

        result.push_str(&transformed);
        result.push('\n');
    }

    result
}

fn main() {
    let source = r#"(module
  (type $Box (struct (field $val (mut string))))
  (global $box (export "box") (ref $Box) (struct.new $Box "hello"))
)"#;

    let transformed = transform_string_types(source);
    println!("=== TRANSFORMED WAT ===");
    println!("{}", transformed);
    println!("=== END ===");

    // Test the transform function directly
    let line = r#"  (global $box (export "box") (ref $Box) (struct.new $Box "hello"))"#;
    println!("\n=== TEST LINE TRANSFORM ===");
    println!("Input:  {}", line);
    println!("Output: {}", transform_string_literal_in_line(line));
}
