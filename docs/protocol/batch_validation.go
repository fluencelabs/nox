package protocol

type ValidationFluenceContract struct {
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

// initializes snapshot metadata and links the associated batch validation endorsement
func (contract ValidationFluenceContract) EndorseInit(height int64, seal Seal, meta SnapshotMeta) { panic("") }

// adds batch validator's endorsement to the confirmations list
func (contract ValidationFluenceContract) Endorse(height int64, seal Seal) { panic("") }

type BatchValidator struct {
  PublicKey  PublicKey
  privateKey PrivateKey
}

type Subchain struct {
  Manifests    []Manifest
  Transactions []Transactions
}

// fetches the subchain by tracing back starting from the checkpoint with the specified block height
func (validator BatchValidator) FetchSubchain(contract SideFluenceContract, height int64) Subchain {
  var checkpoint = contract.CheckpointsByHeight[height]

  var count = contract.CheckpointInterval + 2
  var manifests = make([]Manifest, count)
  var txss = make([]Transactions, count)

  var receipt = checkpoint.Receipt
  for i := count - 1; i >= 0; i-- {
    manifests[i] = ManifestUnpack(SwarmDownload(receipt))
    txss[i] = TransactionsUnpack(SwarmDownload(manifests[i].TxsReceipt))

    receipt = manifests[i].LastManifestReceipt
  }

  return Subchain{Manifests: manifests, Transactions: txss}
}

// uploads the VM state to Swarm if needed and endorses it in the validation smart contract
func (validator BatchValidator) Endorse(contract ValidationFluenceContract, height int64, state VMState) {
  var swarmHash = SwarmMerkleHash(state.Memory)
  var vmStateHash = MerkleHash(state.Memory)

  var seal = Sign(validator.PublicKey, validator.privateKey, Hash(pack(swarmHash, vmStateHash)))

  _, exists := contract.Confirmations[height]
  if exists {
    // no need to upload the virtual machine snapshot to Swarm
    contract.Endorse(height, seal)
  } else {
    // uploading the state to Swarm
    var receipt = SwarmUpload(state.Memory)

    var meta = SnapshotMeta{SnapshotReceipt: receipt, VMStateHash: vmStateHash}
    contract.EndorseInit(height, seal, meta)
  }
}

// returns the downloaded state and it's correctness status
func (validator BatchValidator) LoadSnapshot(contract ValidationFluenceContract, height int64) (VMState, bool) {
  var confirmation = contract.Confirmations[height]
  var meta = confirmation.SnapshotMeta

  var state = VMStateUnpack(SwarmDownload(meta.SnapshotReceipt))
  var correct = meta.VMStateHash == MerkleHash(state.Memory)

  return state, correct
}

func (validator BatchValidator) Validate(
    code WasmCode,
    basicContract BasicFluenceContract,
    sideContract SideFluenceContract,
    validationContract ValidationFluenceContract,
    height int64,
) {
  // fetching transactions and the previous snapshot
  var subchain = validator.FetchSubchain(sideContract, height)
  var snapshot, ok = validator.LoadSnapshot(validationContract, height - sideContract.CheckpointInterval)

  if ok {
    var nextSnapshot VMState

    for i := 0; i < len(subchain.Manifests) - 2; i++ {
      // verifying BFT consensus
      var window = [3]Manifest{}
      copy(subchain.Manifests[i:i+2], window[0:3])
      var publicKeys = VerifyVMStateConsensus(basicContract, window)

      // verifying the real-time cluster state progress correctness
      nextSnapshot = NextVMState(code, snapshot, subchain.Transactions[i])
      var vmStateHash = MerkleHash(nextSnapshot.Memory)
      if vmStateHash != subchain.Manifests[i].VMStateHash {
        // TODO: dispute state advance using publicKeys, stop processing
        _ = publicKeys
      }
    }

    // uploading the snapshot and sending a signature to the smart contract
    validator.Endorse(validationContract, height, nextSnapshot)
  } else {
    // TODO: dispute snapshot incorrectness
  }
}

// confirms that the transition from the previous virtual machine state to the next state is correct
func (validator BatchValidator) ConfirmTransition(
  prevVMHash Digest, vmHash Digest, txsHash Digest) Seal {
    return Sign(
      validator.PublicKey,
      validator.privateKey,
      Hash(pack(prevVMHash, vmHash, txsHash)),
    )
}

// opens a new snapshot hash mismatch dispute
func (contract ValidationFluenceContract) OpenSnapshotDispute(height int64, chunkIndex int) SnapshotDispute {
  return SnapshotDispute{
    SnapshotMeta: contract.Confirmations[height].SnapshotMeta,
    ChunkIndex:   chunkIndex,
  }
}

type SnapshotDispute struct {
  SnapshotMeta SnapshotMeta
  ChunkIndex int
}

// returns whether the supplied Merkle proofs have passed an audite
func (dispute SnapshotDispute) Audit(memoryRegion []byte, vmProof MerkleProof, swarmProof MerkleProof) bool {
  return VerifyMerkleProof(memoryRegion, vmProof, dispute.SnapshotMeta.VMStateHash) &&
      VerifySwarmProof(memoryRegion, swarmProof, dispute.SnapshotMeta.SnapshotReceipt.ContentHash)
}
