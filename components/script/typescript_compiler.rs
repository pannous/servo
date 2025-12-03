// Copyright 2025 The Servo Project Developers.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! TypeScript to JavaScript compilation using Oxc (Oxidation Compiler)

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::OnceLock;

use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use parking_lot::RwLock;

/// Error type for TypeScript compilation
#[derive(Debug)]
pub enum CompileError {
    ParseError(String),
    TransformError(String),
    CodegenError(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::ParseError(msg) => write!(f, "TypeScript parse error: {}", msg),
            CompileError::TransformError(msg) => write!(f, "TypeScript transform error: {}", msg),
            CompileError::CodegenError(msg) => write!(f, "JavaScript codegen error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// Simple in-memory cache for compiled TypeScript
/// Maps hash(source_code) -> compiled JavaScript
fn get_cache() -> &'static RwLock<HashMap<u64, String>> {
    static CACHE: OnceLock<RwLock<HashMap<u64, String>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Compile TypeScript source code to JavaScript
///
/// # Arguments
/// * `source` - The TypeScript source code
/// * `filename` - The name of the file (for error reporting)
///
/// # Returns
/// Compiled JavaScript code or CompileError
pub fn compile_typescript_to_js(source: &str, filename: &str) -> Result<String, CompileError> {
    log::info!("TypeScript: Compiling {} ({} bytes)", filename, source.len());

    // Check cache first
    let cache_key = calculate_hash(source);
    {
        let cache = get_cache().read();
        if let Some(cached) = cache.get(&cache_key) {
            log::info!("TypeScript: Cache hit for {}", filename);
            return Ok(cached.clone());
        }
    }

    // Compile TypeScript to JavaScript
    let compiled = compile_typescript_internal(source, filename)?;
    log::info!("TypeScript: Successfully compiled {} to {} bytes of JS", filename, compiled.len());

    // Store in cache
    {
        let mut cache = get_cache().write();
        // Limit cache size to 1000 entries
        if cache.len() > 1000 {
            cache.clear();
        }
        cache.insert(cache_key, compiled.clone());
    }

    Ok(compiled)
}

/// Internal compilation function using Oxc
fn compile_typescript_internal(source: &str, filename: &str) -> Result<String, CompileError> {
    // Create allocator for Oxc
    let allocator = Allocator::default();

    // Determine source type (TypeScript or TSX)
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::default().with_typescript(true));

    // Parse the TypeScript code
    let parser_ret = Parser::new(&allocator, source, source_type).parse();

    // Check for parse errors
    if !parser_ret.errors.is_empty() {
        let error_msgs: Vec<String> = parser_ret
            .errors
            .iter()
            .map(|e| format!("{}", e))
            .collect();
        return Err(CompileError::ParseError(error_msgs.join("; ")));
    }

    let mut program = parser_ret.program;

    // Build semantic information (required for transformation)
    let semantic_ret = SemanticBuilder::new()
        .build(&program);

    // Configure transform options to strip TypeScript
    let transform_options = TransformOptions::default();

    // Apply TypeScript stripping transform
    let path = Path::new(filename);
    let transform_result = Transformer::new(&allocator, path, &transform_options)
        .build_with_scoping(semantic_ret.semantic.into_scoping(), &mut program);

    if !transform_result.errors.is_empty() {
        let error_msgs: Vec<String> = transform_result
            .errors
            .iter()
            .map(|e| format!("{}", e))
            .collect();
        return Err(CompileError::TransformError(error_msgs.join("; ")));
    }

    // Generate JavaScript code
    let codegen_options = CodegenOptions::default();
    let codegen_result = Codegen::new().with_options(codegen_options).build(&program);

    Ok(codegen_result.code)
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
    fn test_simple_typescript() {
        let source = r#"
            const greeting: string = "Hello, TypeScript!";
            console.log(greeting);
        "#;

        let result = compile_typescript_to_js(source, "test.ts");
        assert!(result.is_ok());

        let js = result.unwrap();
        assert!(js.contains("Hello, TypeScript!"));
        assert!(!js.contains(": string")); // Type annotation should be stripped
    }

    #[test]
    fn test_function_with_types() {
        let source = r#"
            function add(a: number, b: number): number {
                return a + b;
            }
        "#;

        let result = compile_typescript_to_js(source, "test.ts");
        assert!(result.is_ok());

        let js = result.unwrap();
        assert!(js.contains("function add"));
        assert!(!js.contains(": number")); // Type annotations should be stripped
    }

    #[test]
    fn test_caching() {
        clear_cache();

        let source = "const x: number = 42;";

        // First compilation
        let result1 = compile_typescript_to_js(source, "test.ts");
        assert!(result1.is_ok());

        // Second compilation (should hit cache)
        let result2 = compile_typescript_to_js(source, "test.ts");
        assert!(result2.is_ok());

        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_invalid_typescript() {
        let source = "const x: = 42;"; // Invalid syntax

        let result = compile_typescript_to_js(source, "test.ts");
        assert!(result.is_err());
    }
}
