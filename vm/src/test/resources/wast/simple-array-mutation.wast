;; this example adds 1 to each byte in supplied string

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

    ;; char* invoke(const char *array, int arraySize) {
    ;;   for(int i = 0; i < stringSize; ++i) {
    ;;     ++array[i];
    ;;   }
    ;;
    ;;   return array;
    ;; }
    (func (export "invoke") (param $array i32) (param $arraySize i32) (result i32)
        (local $arrayIdx i32)
        (loop $label$0
            (i32.store8
                (i32.add (get_local $array) (get_local $arrayIdx))
                (i32.add
                    (i32.load8_s
                        (i32.add (get_local $array) (get_local $arrayIdx))
                        )
                    (i32.const 1)
                )
            )
            (br_if $label$0
                (i32.ne
                    (tee_local $arrayIdx
                        (i32.add (get_local $arrayIdx) (i32.const 1))
                    )
                    (get_local $arraySize)
                )
            )
        )

        (call $putArrayResult
            (get_local $array)
            (get_local $arraySize)
            (i32.const 1048592)
        )
    )

    ;; int putArrayResult(const char *string, int stringSize, int address) {
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
    (func $putArrayResult (param $string i32) (param $stringSize i32) (param $address i32) (result i32)
        (local $3 i32)
        (local $4 i32)

        (i32.store8
            (get_local $address)
            (get_local $stringSize)
        )
        (i32.store8
            (i32.add (get_local $address) (i32.const 1))
            (i32.shr_u (get_local $stringSize) (i32.const 8))
        )
        (i32.store8
            (i32.add (get_local $address) (i32.const 2))
            (i32.shr_u (get_local $stringSize) (i32.const 16))
        )
        (i32.store8
            (i32.add (get_local $address) (i32.const 3))
            (i32.shr_u (get_local $stringSize) (i32.const 24))
        )

        (set_local $3 (get_local $address))
        (set_local $address (i32.add (get_local $address) (i32.const 4)))

        (loop $label$0
            ;; globalBuffer[address + 4 + i] = string[i];
            (i32.store8
                (get_local $address)
                (i32.load8_u (get_local $string))
            )

            ;; ++string
            (set_local $string
                (i32.add (get_local $string) (i32.const 1))
            )

            ;; ++globalBuffer
            (set_local $address
                (i32.add (get_local $address) (i32.const 1))
            )

            (br_if $label$0
                (i32.ne
                    (tee_local $4 (i32.add (get_local $4) (i32.const 1)))
                    (get_local $stringSize)
                )
            )
        )

        (get_local $3)
    )
)
