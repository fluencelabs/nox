;; this example simply returns product of two integers

(module $MulModule

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

    ;; int extractInt(const char *buffer, int begin, int end) {
    ;;   int value = 0;
    ;;   const int size = end - begin;
    ;;
    ;;   for(int byteIdx = 0; byteIdx < size; ++byteIdx) {
    ;;     value |= (buffer[begin + byteIdx] << (3 - byteIdx) * 8) & 0xFF;
    ;;   }
    ;;
    ;;   return value;
    ;; }
    (func $extractInt (param $buffer i32) (param $begin i32) (param $end i32) (result i32)
        (local $3 i32)
        (block $label$0
            (br_if $label$0
                (i32.lt_s
                    (tee_local $3
                        (i32.sub (get_local $end) (get_local $begin))
                    )
                    (i32.const 1)
                )
            )
            (set_local $begin
                (i32.add (get_local $buffer) (get_local $begin))
            )
            (set_local $buffer (i32.const 0))
            (set_local $end (i32.const 24))

            (loop $label$1
                (set_local $buffer
                    (i32.or
                        (i32.and
                            (i32.shl
                                (i32.load8_s (get_local $begin))
                                (get_local $end)
                            )
                            (i32.const 255)
                        )
                        (get_local $buffer)
                    )
                )
                (set_local $end
                    (i32.add (get_local $end) (i32.const -8))
                )
                (set_local $begin
                    (i32.add (get_local $begin) (i32.const 1))
                )

                (br_if $label$1
                    (tee_local $3
                        (i32.add (get_local $3) (i32.const -1))
                    )
                )
            )
            (return (get_local $buffer))
        )
        (i32.const 0)
    )

    ;; int mul(const char *buffer, int size) {
    ;;   if(size != 8) {
    ;;     return 0;
    ;;   }
    ;;
    ;;   const int a = extractInt(buffer, 0, 4);
    ;;   const int b = extractInt(buffer, 4, 8);
    ;;
    ;;   return a * b;
    ;; }
    (func (export "sum") (param $buffer i32) (param $size i32) (result i32)
        (local $2 i32)
        (set_local $2 (i32.const 0))

        (block $label$0
            (br_if $label$0
                (i32.ne (get_local $size) (i32.const 8))
            )
            (set_local $2
                (i32.mul
                    (call $extractInt
                        (get_local $buffer)
                        (i32.const 0)
                        (i32.const 4)
                    )
                    (call $extractInt
                        (get_local $buffer)
                        (i32.const 4)
                        (i32.const 8)
                    )
                )
            )
        )
        (call $putIntResult (get_local $2))
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
