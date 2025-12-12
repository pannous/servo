(module
  (type $string (array (mut i8)))
  (data $hello "hello")
  (func $test (export "test") (result (ref $string))
    (array.new_data $string $hello (i32.const 0) (i32.const 5))
  )
)
