;; this example has some functions that recieve and put strings

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (data (i32.const 128) "Hello from Fluence!\00")

    (func (export "allocate") (param $0 i32) (result i32)
        ;; just return constant offset in ByteBuffer
        (i32.const 10000)
    )

    (func (export "deallocate") (param $0 i32) (result i32)
        ;; in this simple example deallocation function does nothing
        (i32.const 10000)
    )

    (func (export "putStringResult") (result i32)
        (call $putStringResult
            (i32.const 128)
            (i32.const 3)
            (i32.const 1048576)
        )
    )

    ;; int putStringResult(const char *string, int stringSize, int address) {
    ;;
    ;;   globalBuffer[address] = (stringSize >> 24) & 0xFF;
    ;;   globalBuffer[address + 1] = (stringSize >> 16) & 0xFF;
    ;;   globalBuffer[address + 2] = (stringSize >> 8) & 0xFF;
    ;;   globalBuffer[address + 3] = stringSize & 0xFF;
    ;;
    ;;   for(int i = 0; i < stringSize; ++i) {
    ;;     globalBuffer[address + 4 + i] = string[i];
    ;;   }
    ;; }
    (func $putStringResult (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
        (i32.store8
            (i32.add (get_local $2) (i32.const 17))
            (i32.shr_u (get_local $1) (i32.const 16))
        )
        (i32.store8
            (i32.add (get_local $2) (i32.const 16))
            (i32.shr_u (get_local $1) (i32.const 24))
        )
        (i32.store8
            (i32.add (get_local $2) (i32.const 18))
            (i32.shr_u (get_local $1) (i32.const 8))
        )
        (i32.store8
            (i32.add (get_local $2) (i32.const 19))
            (get_local $1)
        )

        (set_local $2 (i32.add (get_local $2) (i32.const 20)))

        (loop $label$0 (result i32)
            (i32.store8 (get_local $2)
                (i32.load8_u (get_local $0))
            )
            (set_local $0
                (i32.add (get_local $0) (i32.const 1))
            )
            (set_local $2
                (i32.add (get_local $2) (i32.const 1))
            )
            (br $label$0)
        )
    )
)
