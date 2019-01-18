;; this example has "bad" allocation function that returns offset out of ByteBuffer limits as i64

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32)  (result i64)
        ;; returns maximum value of signed 64-bit integer that wittingly exceeds maximum ByteBuffer size
        ;; (and the address space limit on amd64 architecture)
        (i64.const 9223372036854775807)
    )

    (func (export "deallocate") (param $address i32) (param $size i32) (result i32)
        ;; in this simple deallocation function returns 0
        (i32.const 0)
    )

    (func (export "invoke") (param $0 i32 ) (param $1 i32) (result i32)
        ;; simply returns 10000
        (i32.const 10000)
    )

)
