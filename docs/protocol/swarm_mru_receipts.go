package protocol

// updates the resource associated with the specific resource key
func SwarmMRUUpdate(meta *SwarmMeta) SwarmReceipt { panic("") }

func SwarmMRUReceiptsExample() {
  // data
  var meta SwarmMeta  // some mutable content

  // rules
  // ∀ meta:
    var receipt = SwarmMRUUpdate(&meta)
    assertEq(receipt.ContentHash, SwarmHash(pack(meta)))
}
