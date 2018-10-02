;; this example calculates xor of supplied buffer

(module
    ;; force asmble to use memory
    (memory $0 20)
    (table 0 anyfunc)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32 ) (result i32)
        ;; random offset in ByteBuffer
        (i32.const 10000)
    )
    (func (export "deallocate") (param $0 i32 ) (result i32)
        (i32.const 10000)
    )

    ;; int test(const char *buffer, int size) {
    ;;   int value = 0;
    ;;
    ;;   for(int byteId = 0; byteId < size; ++byteId) {
    ;;     value ^= buffer[byteId];
    ;;   }
    ;;
    ;;   return value;
    ;; }
    (func (export "test") (param $0 i32 ) (param $1 i32) (result i32)
        (local $2 i32)
        (set_local $2 (i32.const 0) )
        (block $label$0
            (br_if $label$0
                (i32.lt_s (get_local $1) (i32.const 1) )
            )

        (loop $label$1
            (set_local $2
                (i32.xor (get_local $2) (i32.load8_s (get_local $0) ) )
            )
            (set_local $0
                (i32.add (get_local $0) (i32.const 1) )
            )
            (br_if $label$1
                (tee_local $1
                    (i32.add (get_local $1) (i32.const -1) )
                )
            )
        )
        )
    (get_local $2)
    )
)
