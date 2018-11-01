package protocol

import "reflect"

type Chunk = []byte

// panics if the condition is false
func assertTrue(condition bool) {
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
func packMulti(object interface{}) []Chunk {
  panic("not implemented!")
}

// extracts keys from the dictionary: accepts map[K]V, returns []K
func keys(dict interface{}) interface{} {
  var m = dict.(map[interface{}]interface{})
  var keys = make([]interface{}, 0, len(m))
  for k := range m {
    keys = append(keys, k)
  }
  return keys
}
