package protocol

type SwarmReceipt struct {
  ContentHash Digest  // Swarm hash of the stored content
  Insurance   Seal    // insurance written by the Swarm node for the accepted content
}

// uploads the content to the Swarm network, returns a receipt of responsibility
func SwarmUpload(content []byte) SwarmReceipt { panic("") }


func SwarmReceiptsExample() {
  // data
  var content []byte // some content

  // rules
  // ∀ content:
    var receipt = SwarmUpload(content)

    assert(receipt.ContentHash == SwarmHash(content))
    assert(SwarmVerify(receipt.Insurance, receipt.ContentHash))
}
