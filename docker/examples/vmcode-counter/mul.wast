;; equivalent to `int mul(int a, int b) { return a * b; }` in C++

(module $multiplier
    (func (export "mul") (param $0 i32 ) (param $1 i32) (result i32)
        (i32.mul (get_local 0) (get_local 1))
    )
)
