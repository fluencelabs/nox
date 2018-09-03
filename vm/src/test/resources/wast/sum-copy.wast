;; equivalent to `int sum(int a, int b) { return a + b; }` in C++

(module
    (func (export "sum") (param $0 i32 ) (param $1 i32) (result i32)
        (i32.add (get_local 0) (get_local 1))
    )
)
