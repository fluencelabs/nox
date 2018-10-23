package protocol

func FetchSubchain(sideContract SideContract, index int) ([]Manifest, []Transactions) {
  var prevCheckpoint = sideContract.Checkpoints[index - 1]
  var checkpoint = sideContract.Checkpoints[index]

  var count = checkpoint.Height - prevCheckpoint.Height
  var manifests = make([]Manifest, count)
  var txss = make([][]Transaction, count)

  var receipt = checkpoint.Receipt
  for i := count - 1; i >= 0; i-- {
    manifests[i] = ManifestUnpack(SwarmDownload(receipt))
    txss[i] = TransactionsUnpack(SwarmDownload(manifests[i].TxsReceipt))

    receipt = manifests[i].LastManifestReceipt
  }

  return manifests, txss
}
