package protocol

func FetchSubchain(sideContract SideContract, index int) ([]Manifest, []Transactions) {
  var checkpoint = sideContract.Checkpoints[index]

  var count = sideContract.CheckpointInterval + 2
  var manifests = make([]Manifest, count)
  var txss = make([]Transactions, count)

  var receipt = checkpoint.Receipt
  for i := count - 1; i >= 0; i-- {
    manifests[i] = ManifestUnpack(SwarmDownload(receipt))
    txss[i] = TransactionsUnpack(SwarmDownload(manifests[i].TxsReceipt))

    receipt = manifests[i].LastManifestReceipt
  }

  return manifests, txss
}

type ValidationContract struct {
  Confirmations map[int64]Confirmation // confirmations: block height –> confirmation
}

type Confirmation struct {
  SnapshotMeta SnapshotMeta // virtual machine state metadata
  Endorsements []Seal       // batch validators signatures certifying snapshot correctness
}

type SnapshotMeta struct {
  SnapshotReceipt SwarmReceipt // Swarm receipt for the virtual machine state snapshot
  VMStateHash     Digest       // virtual machine state hash
}

// adds batch validator's endorsement to the confirmations list
func (contract ValidationContract) Endorse(height int64, seal Seal, meta *SnapshotMeta) { panic("") }

type BatchValidator struct {
  PublicKey  PublicKey
  PrivateKey PrivateKey
}

func (validator BatchValidator) Endorse(contract ValidationContract, height int64, state VMState) {
  var swarmHash = SwarmHash(pack(state.Chunks))
  var vmStateHash = MerkleRoot(state.Chunks)

  var seal = Sign(validator.PublicKey, validator.PrivateKey, Hash(pack(swarmHash, vmStateHash)))

  _, exists := contract.Confirmations[height]
  if exists {
    contract.Endorse(height, seal, nil)
  } else {
    var meta = SnapshotMeta{SnapshotReceipt: SwarmUpload(pack(state.Chunks)), VMStateHash: vmStateHash}
    contract.Endorse(height, seal, &meta)
  }
}

func LoadSnapshot(contract ValidationContract, height int64) (VMState, bool) {
  var confirmation = contract.Confirmations[height]
  var meta = confirmation.SnapshotMeta

  var state = VMStateUnpack(SwarmDownload(meta.SnapshotReceipt))
  if meta.VMStateHash != MerkleRoot(state.Chunks) {
    return VMState{}, false
  } else {
    return state, true
  }
}

// opens a new hash mismatch dispute
func (contract ValidationContract) OpenHashMismatchDispute(height int64, chunkIndex int) HashMismatchDispute {
  return HashMismatchDispute{
    SnapshotMeta: contract.Confirmations[height].SnapshotMeta,
    ChunkIndex:   chunkIndex,
  }
}

type HashMismatchDispute struct {
  SnapshotMeta SnapshotMeta
  ChunkIndex int
}

// returns whether the supplied Merkle proofs have passed an audite
func (dispute HashMismatchDispute) Audit(chunk Chunk, vmProof MerkleProof, swarmProof MerkleProof) bool {
  // TODO: check chunk index in the proof
  // TODO: use Swarm-based Merkle proof verification

  return VerifyMerkleProof(chunk, vmProof, dispute.SnapshotMeta.VMStateHash) &&
      VerifyMerkleProof(chunk, swarmProof, dispute.SnapshotMeta.SnapshotReceipt.ContentHash)
}
