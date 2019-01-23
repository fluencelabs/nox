;; this example has "bad" allocation function that returns offset out of ByteBuffer limits as f64

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32)  (result f64)
        ;; returns floating-point f64 number instead od integer
        (f64.const 200000000.12345)
    )

    (func (export "deallocate") (param $address i32) (param $size i32) (return)
        ;; in this simple example deallocation function does nothing
        (drop)
        (drop)
    )

    (func (export "invoke") (param $0 i32 ) (param $1 i32) (result i32)
        ;; simply returns 10000
        (i32.const 10000)
    )

)
