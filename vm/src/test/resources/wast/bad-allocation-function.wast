;; this example has "bad" allocation function that returns offset out if ByteBuffer limits

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32  (result i64)
        ;; returns maximum value of signed 64-bit integer that wittingly exceeds maximum ByteBufer size
        ;; (and the address space limit on amd64 architecture)
        (i64.const 9223372036854775807)
    )

    (func (export "deallocate") (param $0 i32 ) (result i32)
        ;; in this simple example deallocation function does nothing
    )

    (func (export "test") (param $0 i32 ) (param $1 i32) (result i32)
        ;; simply returns 10000
        (call 3 (i32.const 10000))
    )

    ;; int putResultInt(int result) {
    ;;   const int address = 1024*1024;
    ;;
    ;;   globalBuffer[address] = 0;
    ;;   globalBuffer[address + 1] = 0;
    ;;   globalBuffer[address + 2] = 0;
    ;;   globalBuffer[address + 3] = 4;
    ;;
    ;;   for(int i = 0; i < 4; ++i) {
    ;;     globalBuffer[address + 4 + i ] = ((result >> 8*i) & 0xFF);
    ;;   }
    ;;
    ;;   return address;
    ;; }
    (func (export "putResultInt") (param $0 i32) (result i32)
        (local $1 i32)
        (local $2 i32)
        (set_local $2 (i32.const 0))
        (i32.store offset=1048592 (i32.const 0) (i32.const 4))
        (set_local $1 (i32.const 1048596))
        (loop $label$0
            (i32.store8
                (get_local $1)
                (i32.shr_u (get_local $0) (get_local $2))
            )
            (set_local $1
                (i32.add (get_local $1) (i32.const 1))
            )
            (br_if $label$0
                (i32.ne
                    (tee_local $2 (i32.add (get_local $2) (i32.const 8)))
                    (i32.const 32)
                )
            )
        )
        (i32.const 1048592)
    )
)
