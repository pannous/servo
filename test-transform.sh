#!/bin/bash
# Quick test to see transformation output
cat > /tmp/test-wat.txt <<'EOF'
(module
  (type $Box (struct (field $val (mut string))))
  (global $box (export "box") (ref $Box) (struct.new $Box "hello"))
)
EOF

# This will trigger the transformation and show logs
servox test-string-simple.html 2>&1 | grep -A50 "Transformed WAT" | head -60
