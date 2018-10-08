;; this example has "bad" allocation function that returns offset out if ByteBuffer limits

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (table 0 anyfunc)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32 ) (result i64)
        ;; returns maximum value of signed 64-bit integer that wittingly exceeds maximum ByteBufer size
        ;; (and the address space limit on amd64 architecture)
        (i64.const 9223372036854775807)
    )
    (func (export "deallocate") (param $0 i32 ) (result i32)
        ;; in this simple example deallocation function does nothing
        (i32.const 10000)
    )

    (func (export "test") (param $0 i32 ) (param $1 i32) (result i32)
        ;; simply returns 10000
        (i32.const 10000)
    )
)
