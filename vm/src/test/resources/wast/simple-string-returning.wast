;; this example has some functions that recieve and put strings

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32) (result i32)
        ;; just return constant offset in ByteBuffer
        (i32.const 10000)
    )

    (func (export "deallocate") (param $0 i32) (result i32)
        ;; in this simple example deallocation function does nothing
        (i32.const 10000)
    )
)
