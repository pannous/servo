# Servo HTML Parser: Investigation Answers

## Question 1: What makes `<script type="text/typescript">` parse successfully while `<script type="application/wasm">` hangs?

### Answer

**TypeScript works** because:
- `type="text/typescript"` starts with the `text/` prefix
- html5ever's TreeBuilder has heuristic handling for `text/*` MIME types
- As a safety measure, html5ever enters **RawData mode** for all `text/*` types
- Content is consumed literally until `</script>` tag
- This raw content reaches Servo, which detects it as TypeScript
- Servo's TypeScript compiler converts it to JavaScript
- Execution proceeds normally

**WASM hangs** because:
- `type="application/wasm"` is not recognized by html5ever
- html5ever does NOT enter RawData mode for unknown types
- Instead, html5ever continues normal HTML parsing
- WAT content `(module ...)` is parsed as HTML
- The `(` character is invalid in HTML tag context
- Tokenizer enters error recovery mode and hangs indefinitely

## Question 2: How does html5ever's tokenizer handle different script type attributes?

### Answer

html5ever's TreeBuilder checks the `type` attribute of script elements and decides tokenizer state:

```rust
// Pseudocode of html5ever's behavior
fn process_script_start_tag(type_attr: Option<String>) {
    let mime_type = type_attr.unwrap_or("text/javascript");
    
    // Check if it's a JavaScript-like MIME type
    if is_javascript_like_mime(mime_type) {
        // Enter RawData mode (actually called ScriptData state)
        return TokenizerState::RawData;
    } else if mime_type.starts_with("text/") {
        // SAFETY: Text types might be scripts, enter RawData as fallback
        return TokenizerState::RawData;
    } else {
        // Unknown type, continue normal HTML parsing
        return TokenizerState::Data;  // ❌ PROBLEM: WAT is invalid HTML
    }
}
```

**Recognized JavaScript MIME types:**
- text/javascript
- application/javascript
- text/ecmascript
- application/ecmascript
- Legacy types: text/jscript, text/livescript, etc.

**Recognized for safety (text/* prefix):**
- Appears to work for text/typescript
- Does NOT work for application/wasm (no text/ prefix!)

## Question 3: Is there something special about "text/*" mime types vs "application/*"?

### Answer

**Yes, absolutely.**

html5ever treats `text/*` types specially:
- `text/*` types are assumed to be potentially script-like
- html5ever enters RawData mode as a safety precaution
- This is why `type="text/typescript"` works
- Any `text/*` type gets raw text treatment

But `application/*` types:
- Must be explicitly recognized as JavaScript-like
- `application/javascript` is explicitly recognized ✅
- `application/wasm` is NOT explicitly recognized ❌
- Falls through to normal HTML parsing
- Causes WAT content to be parsed as malformed HTML
- Results in hang

**Why this design?**
- HTML spec says script types are MIME types
- For security, treat unknown types as potential scripts
- `text/*` types are assumed to be text-based scripts
- But `application/*` types can be anything (images, PDFs, etc.)
- So html5ever only treats explicitly known `application/*` types as scripts

## Question 4: Check if there's any special handling for TypeScript that we're missing for WASM

### Answer

**No special handling for TypeScript in the parser level.**

TypeScript works through:
1. **Parser level:** html5ever recognizes `text/*` and enters RawData ✓
2. **Servo level:** Script type detection recognizes `text/typescript` ✓
3. **Compiler level:** typescript_compiler converts TS → JS ✓

**WASM should work similarly, but doesn't because:**
1. **Parser level:** html5ever does NOT recognize `application/wasm` ✗
   - Cannot enter RawData mode
   - Content parsed as HTML, hangs ✗

The special handling for TypeScript is minimal:
- Lines 1337-1339 in htmlscriptelement.rs: Detect as ScriptType::TypeScript
- Lines 259-278 in htmlscriptelement.rs: Call typescript_compiler

For WASM:
- Lines 1346-1348 in htmlscriptelement.rs: Detect as ScriptType::Wasm
- Lines 279-295 in htmlscriptelement.rs: Call wasm_compiler
- **BUT NEVER REACHED** because parser hangs before content is parsed

### Detailed Code Comparison

**TypeScript path:**
```
HTML: <script type="text/typescript">const x: number = 42;</script>
  ↓ (html5ever recognizes text/*)
enters RawData mode ✓
  ↓ (content consumed literally)
Servo receives: "const x: number = 42;"
  ↓ (script type detection)
gets_script_type() → Some(ScriptType::TypeScript)
  ↓ (compilation)
ScriptOrigin::internal() → typescript_compiler::compile_typescript_to_js()
  ↓
Executes as JavaScript ✓
```

**WASM path (broken):**
```
HTML: <script type="application/wasm">(module ...)</script>
  ↓ (html5ever doesn't recognize application/wasm)
does NOT enter RawData mode ✗
  ↓ (content parsed as HTML!)
Tokenizer sees: "(module" and tries to parse as tag
  ↓
Invalid HTML syntax, error recovery
  ↓
HANGS waiting for malformed tag to close
  ↓ (never reached)
Script type detection never runs ✗
Script compilation never runs ✗
```

## Summary of Root Causes

| Aspect | TypeScript | WASM | Status |
|--------|-----------|------|--------|
| Parser recognizes type | ✓ (text/*) | ✗ (app/*) | Different |
| Enters RawData mode | ✓ Yes | ✗ No | Different |
| Content parsed correctly | ✓ Yes | ✗ As HTML | Different |
| Script detection runs | ✓ Yes | ✗ Hangs first | Different |
| Compilation runs | ✓ Yes | ✗ Never | Different |
| Final result | ✓ Works | ✗ Hangs | Different |

## The Critical Insight

**The problem is NOT in Servo's script handling code.**

Servo has full support for:
- TypeScript detection ✓
- TypeScript compilation ✓
- WASM detection ✓
- WASM compilation ✓

**The problem IS in html5ever's tokenizer.**

html5ever's TreeBuilder doesn't recognize `application/wasm` as requiring RawData mode, so the content never reaches Servo's script handling code. The parser hangs before Servo ever gets a chance to process the script element.

## Fix Location

The fix must be at the html5ever level:
1. Modify html5ever's MIME type recognition to include `application/wasm`
2. Modify html5ever's MIME type recognition to include `text/wasm`
3. Optionally add `text/typescript` for clarity

Or in Servo:
1. Pre-process HTML before passing to html5ever
2. Replace `type="application/wasm"` with `type="text/wasm"` or `type="text/javascript"`
3. Restore original type after parsing

The core issue is that html5ever, not Servo, controls when RawData mode is entered, and html5ever doesn't recognize WASM MIME types.
