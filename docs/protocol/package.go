package protocol

// panics if the condition is false
func assert(condition bool) {
  if !condition {
    panic("failed assertion!")
  }
}

// serializes arguments to a byte array
func pack(args... interface{}) []byte {
  panic("not implemented!")
}
