;; Simple in-memory counter

(module

 (memory $0 1)
 (table 0 anyfunc)
 (export "memory" (memory $0))

 (export "inc" (func $inc))
 (export "get" (func $get))

 ;;
 ;; Initializes counter with zero value
 ;;
 (data (i32.const 12) "\00\00\00\00")

 ;;
 ;; Increments variable in memory by one.
 ;;
  (func $inc (; 0 ;)
   (i32.store offset=12
    (i32.const 0)
    (i32.add
     (i32.load offset=12 (i32.const 0))
     (i32.const 1)
    )
   )
  )

  ;;
  ;; Public fn for getting value of counter.
  ;;
  (func $get (; 1 ;) (result i32)
   (i32.load offset=12 (i32.const 0))
  )

)
