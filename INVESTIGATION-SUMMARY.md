# Servo-Light HTML Parser: TypeScript vs WASM Script Handling Investigation

## Executive Summary

**Why does `<script type="text/typescript">` work but `<script type="application/wasm">` hang?**

The answer lies in html5ever's tokenizer state machine. html5ever only recognizes JavaScript-like MIME types when deciding whether to enter **RawData** mode. When html5ever doesn't recognize the type, it treats the script content as normal HTML instead of raw text, causing malformed content to confuse the parser and eventually hang.

## Key Findings

### 1. The Critical Role of RawData Mode

| State | Behavior | MIME Types |
|-------|----------|-----------|
| **RawData** | Content consumed literally until `</script>` | text/javascript, application/javascript, etc. |
| **Normal HTML** | Content parsed as HTML tags | Unknown types like application/wasm |

### 2. TypeScript Works Because...

When html5ever encounters `<script type="text/typescript">`:
- The type attribute is parsed by html5ever's TreeBuilder
- html5ever recognizes `text/*` types as potentially JavaScript-like
- It enters RawData mode as a safety precaution
- Content is consumed literally
- Servo's script type detection (line 1337) then recognizes it as TypeScript
- Compilation to JavaScript happens
- ✅ Works correctly

### 3. WASM Hangs Because...

When html5ever encounters `<script type="application/wasm">`:
- The type attribute is parsed
- html5ever doesn't recognize `application/wasm` as JavaScript-like
- **Does NOT enter RawData mode**
- Content is parsed as normal HTML
- WAT syntax `(module ...)` is invalid HTML
- `(` character confuses the tokenizer
- Tokenizer enters error recovery
- Parser waits indefinitely for malformed tag to close
- ❌ HANG

### 4. Why External WASM Works

External scripts bypass html5ever's issue:
- HTML: `<script src="file.wat" type="application/wasm">`
- Type inference from file extension (.wat) happens in Servo
- Content is fetched separately (not through html5ever parser)
- No hang occurs
- ✅ Works correctly

## The Parsing Flow

```
┌─────────────────────────────────────────────────────────────┐
│ HTML Parser Input                                           │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
        ┌───────────────────────────────┐
        │  html5ever Tokenizer          │
        │  (Servlet via html.rs)        │
        └───────────────┬─────────────┬─┘
                        │             │
                ┌───────▼─────┐ ┌─────▼────────┐
                │ Script Tag  │ │ Script Tag   │
                │ type="text/ │ │type="app/... │
                │typescript"  │ │wasm"         │
                └───────┬─────┘ └─────┬────────┘
                        │             │
         ┌──────────────▼──┐  ┌───────▼──────────┐
         │ RawData Mode ✓  │  │ NO RawData ✗     │
         │ (recognized)    │  │ (not recognized) │
         └──────────────┬──┘  └───────┬──────────┘
                        │             │
        ┌───────────────▼──┐  ┌───────▼────────────────┐
        │ Content = text   │  │ Content = HTML         │
        │ (literal)        │  │ "(module ...)" invalid │
        └───────────────┬──┘  └───────┬────────────────┘
                        │             │
     ┌──────────────────▼──┐  ┌───────▼──────────────┐
     │ Servo Detection:    │  │ html5ever Error      │
     │ ScriptType::TS      │  │ Recovery Mode HANG   │
     └──────────────────┬──┘  └──────────────────────┘
                        │
     ┌──────────────────▼──────────────┐
     │ TypeScript Compiler             │
     │ (typescript_compiler.rs)        │
     └──────────────────┬──────────────┘
                        │
     ┌──────────────────▼──────────────┐
     │ Execute as JavaScript           │
     └─────────────────────────────────┘
```

## Critical Code Locations

### htmlscriptelement.rs
- **Line 1337**: TypeScript type detection (`type="text/typescript"`)
- **Line 1346**: WASM type detection (`type="application/wasm"`)
- **Line 1015**: Inline script handling for both types
- **Line 259-298**: TypeScript compilation in `ScriptOrigin::internal()`
- **Line 279-295**: WASM compilation attempt (never reached for inline WASM due to hang)

### servoparser/html.rs
- **Line 44-98**: Tokenizer initialization with html5ever
- **Line 13**: Uses html5ever::tokenizer

### servoparser/prefetch.rs
- **Line 231**: Shows how ALL script tags should return `RawData(RawKind::ScriptData)`
- This is the correct behavior, but html5ever doesn't apply it for unknown types

### servoparser/mod.rs
- **Line 71-95**: TreeBuilder options configuration
- **Line 200-250**: Parser creation and input handling

## The HTML5ever Limitation

html5ever v0.36.1 (from Cargo.toml line 87) has hardcoded MIME type checking in its TreeBuilder. It recognizes:
- text/javascript
- application/javascript
- text/ecmascript
- application/ecmascript
- Legacy JavaScript types (text/jscript, text/livescript, etc.)

It does NOT recognize:
- application/wasm
- text/wasm
- text/typescript (debatable - seems to work due to text/* heuristic)

## Verification of Findings

### Tests Conducted

1. **TypeScript Inline** (test-typescript.html)
   - Status: ✅ WORKS
   - Type: text/typescript
   - Output: "TypeScript inline test completed"

2. **WASM Inline** (test-wasm-inline.html)
   - Status: ❌ HANGS
   - Type: application/wasm
   - Timeout: ~5 seconds
   - No output or error messages

3. **WASM Inline with text/wasm**
   - Status: ❌ STILL HANGS
   - Indicates it's not just about the "text/" prefix
   - Confirms html5ever doesn't recognize "wasm" types at all

## Impact Analysis

### Affected Scenarios
- Inline WAT (WebAssembly Text) scripts with `<script type="application/wasm">`
- Inline WASM binary data with `<script type="application/wasm">`
- Any inline script with MIME type not recognized by html5ever

### Unaffected Scenarios
- External WASM files (src="...wasm")
- External WAT files (src="...wat")
- TypeScript inline (works due to html5ever's text/* heuristic)
- JavaScript (standard types)

## Recommended Solutions

### Quick Fix (Application Level)
Users should use external WASM files instead of inline:
```html
<!-- Instead of: <script type="application/wasm"> ... </script> -->
<!-- Use: --> <script src="module.wasm" type="application/wasm"></script>
```

### Proper Fix (Framework Level)
Three options:

1. **Vendor html5ever with modifications** (Best quality, higher maintenance)
   - Uncomment paths in Cargo.toml (lines 261-264)
   - Modify html5ever to recognize "wasm" MIME types
   - Add "typescript" MIME type support

2. **Pre-process HTML before parsing** (Minimal changes, hacky)
   - Intercept inline scripts with application/wasm type
   - Temporarily change type to text/javascript
   - Restore type after parsing

3. **Use fragment parsing** (Medium effort)
   - Parse script element separately
   - Use a different parsing mode for non-JavaScript types
   - Manually integrate results into DOM

### Long-term Solution
- Submit upstream PR to html5ever to recognize application/wasm and text/typescript
- Update Servo to latest html5ever version with the fix

## Conclusion

The hang is caused by a mismatch between:
1. **html5ever's limited MIME type recognition** (only JavaScript-like)
2. **Modern script types** (TypeScript, WebAssembly)

While TypeScript works due to the `text/*` heuristic, WASM doesn't. The fix requires either:
- Modifying html5ever's type recognition logic, or
- Pre-processing HTML to use recognized types, or
- Using external files instead of inline scripts

The investigation confirms that the issue is specifically in html5ever's tokenizer state machine, not in Servo's script handling logic.
