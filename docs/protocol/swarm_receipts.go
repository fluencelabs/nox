package protocol

type SwarmReceipt struct {
  ContentHash Digest  // Swarm hash of the stored content
  Insurance   Seal    // insurance written by the Swarm node for the accepted content
}

// uploads the content to the Swarm network, returns a receipt of responsibility
func SwarmUpload(content []byte) SwarmReceipt { panic("") }

// downloads the content from the Swarm network using the supplied receipt
func SwarmDownload(receipt SwarmReceipt) []byte { panic("") }

func SwarmReceiptsExample() {
  // data
  var content []byte // some content

  // rules
  // ∀ content:
    var receipt = SwarmUpload(content)

    assertEq(receipt.ContentHash, SwarmHash(content))
    assertTrue(SwarmVerify(receipt.Insurance, receipt.ContentHash))
}
