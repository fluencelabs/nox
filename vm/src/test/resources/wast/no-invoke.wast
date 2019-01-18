;; this example has allocate/deallocate functions but doesn't have invoke function.

(module

    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32) (result i32)
        ;; just return constant offset in ByteBuffer
        (i32.const 10000)
    )

    (func (export "deallocate") (param $address i32) (param $size i32) (result i32)
        ;; in this simple deallocation function returns 0
        (i32.const 0)
    )
)
