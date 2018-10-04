;; this example has allocate/deallocate functions but doesn't have memory sections.
;; Asmble version 0.4.0 doesn't generate getMemory function in this case.

(module
    (func (export "allocate") (param $0 i32 ) (result i32)
        ;; just return constant offset in ByteBuffer
        (i32.const 10000)
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
