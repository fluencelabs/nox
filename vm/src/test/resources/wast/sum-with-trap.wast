;; this example simply returns

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32) (result i32)
        ;; just return constant offset in ByteBuffer
        (i32.const 10000)
    )

    (func (export "deallocate") (param $0 i32) (return)
        ;; in this simple example deallocation function does nothing
        (drop)
    )

    ;; int sum(int a, int b) {
    ;;  return a + b;
    ;; }
    (func (export "sum") (param $0 i32) (param $1 i32) (result i32)
        (unreachable) ;; unreachable: An instruction which always traps.
    )
)
