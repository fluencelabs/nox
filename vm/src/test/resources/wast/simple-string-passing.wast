;; this example calculates circular xor of supplied buffer

(module
    ;; force Asmble to use memory
    (memory $0 20)
    (export "memory" (memory $0))

    (func (export "allocate") (param $0 i32) (result i32)
        ;; just return constant offset in ByteBuffer
        (i32.const 10000)
    )

    (func (export "deallocate") (param $address i32) (param $size i32) (return)
        ;; in this simple example deallocation function does nothing
        (drop)
        (drop)
    )

    ;; int invoke(const char *buffer, int size) {
    ;;   int value = 0;
    ;;
    ;;   for(int byteId = 0; byteId < size; ++byteId) {
    ;;     value ^= buffer[byteId];
    ;;   }
    ;;
    ;;   return value;
    ;; }
    (func (export "invoke") (param $buffer i32 ) (param $size i32) (result i32)
        (local $value i32)
        (set_local $value (i32.const 0) )
        (block $label$0
            (br_if $label$0
                (i32.lt_s (get_local $size) (i32.const 1) )
            )

            (loop $label$1
                (set_local $value
                    (i32.xor (get_local $value) (i32.load8_s (get_local $buffer) ) )
                )
                (set_local $buffer
                    (i32.add (get_local $buffer) (i32.const 1) )
                )
                (br_if $label$1
                    (tee_local $size
                        (i32.add (get_local $size) (i32.const -1) )
                    )
                )
            )
        )
        (call $putIntResult (get_local $value))
    )

    ;; int putIntResult(int result) {
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
    (func $putIntResult (param $result i32) (result i32)
        (local $1 i32)
        (local $2 i32)
        (set_local $2 (i32.const 0))
        (i32.store offset=1048592 (i32.const 0) (i32.const 4))
        (set_local $1 (i32.const 1048596))
        (loop $label$0
            (i32.store8
                (get_local $1)
                (i32.shr_u (get_local $result) (get_local $2))
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
