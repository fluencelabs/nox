;; equivalent to `int sum(int a, int b) { return a + b; }` in C++

(module
    (func (export "sum") (param $0 i32 ) (param $1 i32) (result i32)
        (unreachable) ;; unreachable: An instruction which always traps.
    )
)
