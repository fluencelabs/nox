package protocol

func FetchSubchain(sideContract SideContract, index int) ([]Manifest, []Transactions) {
  var checkpoint = sideContract.Checkpoints[index]

  var count = sideContract.CheckpointInterval + 2
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

type ValidationContract struct {
  Snapshots map[int64]SwarmReceipt
}
