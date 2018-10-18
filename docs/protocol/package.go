package protocol

import "reflect"

// panics if the condition is false
func assert(condition bool) {
  if !condition {
    panic("failed assertion!")
  }
}

// panics if the condition is false
func assertEq(x, y interface{}) {
  if !reflect.DeepEqual(x, y) {
    panic("arguments not equal!")
  }
}

// serializes arguments to a byte array
func pack(args... interface{}) []byte {
  panic("not implemented!")
}

// serializes elements of a compound object (array or structure) into byte arrays
func packMulti(object interface{}) [][]byte {
  panic("not implemented!")
}
