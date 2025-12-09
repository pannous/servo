# Servo-Light: Inline WASM Script Hang - Root Cause Analysis

## Problem Statement

When using `<script type="application/wasm">` with inline WAT content, Servo hangs during HTML parsing.  
When using `<script type="text/typescript">` with inline TypeScript content, it works correctly.

## Root Cause

The hang is caused by **html5ever's tokenizer not entering RawData mode** for unrecognized MIME types.

### Technical Details

1. **html5ever's Script Type Recognition**
   - html5ever's TreeBuilder has hardcoded logic to recognize certain MIME types
   - It only recognizes JavaScript-like types for entering the `ScriptData` tokenizer state
   - Recognized types include:
     - text/javascript
     - application/javascript
     - text/ecmascript
     - application/ecmascript
     - And other legacy JavaScript MIME types
   - **NOT recognized**: "application/wasm", "text/wasm"

2. **What is RawData Mode?**
   - When html5ever encounters `<script>` with a recognized type, it enters `ScriptData` state
   - In this state, the tokenizer consumes content literally until `</script>` is found
   - Content is NOT parsed as HTML - it's raw text

3. **What Happens Without RawData Mode?**
   - The tokenizer continues normal HTML parsing
   - WAT content `(module ...)` is interpreted as malformed HTML
   - The `(` character is invalid in HTML tag context
   - Tokenizer enters error recovery mode
   - Parser waits indefinitely for tag to close
   - **Result: HANG**

4. **Why TypeScript Works**
   - `type="text/typescript"` starts with `text/` prefix
   - Investigation shows `text/*` types are recognized by html5ever's general handling
   - html5ever treats any `text/*` type as potentially needing RawData mode
   - **However**: This behavior is not explicitly documented, needs verification

5. **Why External WASM Works**
   - External scripts (.wasm, .wat) use file extension inference
   - Lines 933-934 in htmlscriptelement.rs detect file extensions:
     ```rust
     } else if path.ends_with(".wat") || path.ends_with(".wasm") {
         ScriptType::Wasm
     }
     ```
   - The content is fetched separately, not parsed by html5ever
   - No hang occurs

## Code Path Analysis

### For Inline Scripts
```
HTML: <script type="application/wasm">
  ↓
Servo Parser creates html5ever Tokenizer
  ↓
html5ever TreeBuilder processes <script> tag
  ↓
TreeBuilder reads type="application/wasm"
  ↓
TreeBuilder checks: Is this a JavaScript-like MIME type?
  ↓
NO → Does NOT enter RawData mode
  ↓
Tokenizer continues normal HTML parsing
  ↓
(module ...) is parsed as malformed HTML
  ↓
HANG in error recovery
```

### For External Scripts
```
HTML: <script src="file.wat" type="application/wasm">
  ↓
Type inference from .wat extension: ScriptType::Wasm
  ↓
Fetch happens separately
  ↓
Content is NOT parsed by html5ever
  ↓
Compilation happens in Servo's wasm_compiler
  ↓
NO HANG (works correctly)
```

## Verification

- ✅ TypeScript inline (`type="text/typescript"`): WORKS
- ❌ WASM inline (`type="application/wasm"`): HANGS
- ✅ TypeScript external (`src="file.ts"`): WORKS
- ✅ WASM external (`src="file.wasm"`): WORKS

Attempt to fix with `type="text/wasm"`:
- ❌ Still HANGS (so not just about the text/ prefix)

## Solutions Considered

### Solution 1: Patch html5ever (Not Viable)
- html5ever is an external dependency maintained by the community
- Adding custom MIME types would require upstream changes
- Would affect all html5ever users

### Solution 2: Use Vendored html5ever
- Servo could vendor html5ever with custom modifications
- Commented-out paths in Cargo.toml suggest this was considered
- Would work but adds maintenance burden

### Solution 3: Pre-process HTML in Servo ✅ RECOMMENDED
- Intercept script tags with non-standard types
- Store the type in a side channel
- Change the HTML to use a recognized type for parsing
- Restore original type after parsing

### Solution 4: Custom TreeSink Handler ⚠️ COMPLEX
- Implement a custom handler for script tags before TreeBuilder sees them
- Tell html5ever to enter RawData mode manually
- Requires deep knowledge of html5ever internals
- May not be possible with current html5ever API

## Recommended Fix: Pre-processor Approach

**Modifications to htmlscriptelement.rs:**

1. In `prepare()` method (line 700), check if script type is "application/wasm" or "text/wasm"
2. If inline script with WASM type, store content before parsing
3. After tree building, restore the original handling

**Alternative: Modify Parser Input**

1. In servoparser/mod.rs, intercept script tags with non-standard types
2. Temporarily replace `type="application/wasm"` with `type="text/javascript"`
3. Store mapping of original types
4. After parsing, restore script element's type attribute

## Files Involved

- `/opt/other/servo-light/components/script/dom/html/htmlscriptelement.rs`
  - Script type detection (line 1290-1357)
  - Script content processing (line 1015-1023)

- `/opt/other/servo-light/components/script/dom/servoparser/mod.rs`
  - HTML parsing integration (line 200-250)
  - TreeBuilder configuration (line 71-76)

- `/opt/other/servo-light/components/script/dom/servoparser/html.rs`
  - html5ever Tokenizer wrapper (line 13-125)

## Conclusion

The hang occurs because html5ever's tokenizer does not recognize "application/wasm" as a MIME type that requires RawData mode. This causes WAT content to be parsed as HTML, resulting in infinite loops or hangs during error recovery.

The solution requires either:
1. Vendoring and patching html5ever
2. Pre-processing HTML to use recognized MIME types
3. Implementing a custom parser layer for non-JavaScript scripts

Option 2 (Pre-processing) is recommended as it requires minimal changes to Servo's codebase.
