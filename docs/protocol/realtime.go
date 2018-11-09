package protocol

// verifies that a transaction was originated by the client with enough funds deposited
func VerifyTransaction(contract BasicFluenceContract, tx Transaction, minDeposit int64){
  // checking that the client actually exists in the contract
  var deposit, ok = contract.ClientDeposits[tx.Seal.PublicKey]
  assertTrue(ok)

  // checking that the client has enough funds
  assertTrue(deposit >= minDeposit)

  // checking that the transaction was signed by this client
  assertTrue(Verify(tx.Seal, Hash(tx.Invoke)))
}

// listed Tendermint functions carry the same meaning and arguments as core functions
func TmSign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal { panic("") }
func TmVerify(seal Seal, digest Digest) bool { panic("") }

// splits data in tendermint-sized chunks and computes a Merkle root from them
func TmMerkleHash(chunks []Chunk) Digest { panic("") }

type Block struct {
  Header     Header       // block header
  LastCommit []Seal       // Tendermint nodes votes for the previous block
  Txs        Transactions // transactions as sent by clients
}

type Header struct {
  LastBlockHash  Digest // Merkle root of the previous block header fields
  LastCommitHash Digest // Merkle root of the last commit votes
  TxsHash        Digest // Merkle root of the block transactions
  AppHash        Digest // application state hash after the previous block
}

// Tendermint real-time node
type RealtimeNode struct {
  PublicKey  PublicKey
  privateKey PrivateKey
}

// signs the block assuming the node has voted for it during consensus settlement
func (node RealtimeNode) SignBlockHash(blockHash Digest) Seal {
  return TmSign(node.PublicKey, node.privateKey, blockHash)
}

// prepares the block (assuming the nodes have reached a consensus)
func PrepareBlock(nodes []RealtimeNode, prevBlock Block, txs Transactions, appHash Digest) Block {
  var lastBlockHash = TmMerkleHash(packMulti(prevBlock.Header))
  var lastCommit = make([]Seal, 0, len(nodes))
  for i, node := range nodes {
    lastCommit[i] = node.SignBlockHash(lastBlockHash)
  }

  return Block{
    Header: Header{
      LastBlockHash:  lastBlockHash,
      LastCommitHash: TmMerkleHash(packMulti(lastCommit)),
      TxsHash:        TmMerkleHash(packMulti(txs)),
      AppHash:        appHash,
    },
    LastCommit: lastCommit,
    Txs:        txs,
  }
}

type VMState struct {
  Memory []byte // virtual machine contiguous memory
}

// deserializes a byte array into the virtual machine state
func VMStateUnpack([]byte) VMState { panic("") }

// applies block transactions to the virtual machine state to produce the new state
func NextVMState(code WasmCode, vmState VMState, txs []Transaction) VMState { panic("") }

type Manifest struct {
  Header              Header       // block header
  VMStateHash         Digest       // Merkle root of the VM state derived by applying the block
  LastCommit          []Seal       // Tendermint nodes signatures for the previous block header
  TxsReceipt          SwarmReceipt // Swarm hash of the block transactions
  LastManifestReceipt SwarmReceipt // Swarm hash of the previous manifest
}

// deserializes a byte array into the manifest
func ManifestUnpack([]byte) Manifest { panic("") }

// returns the new virtual machine state, the manifest for the stored block and the next app hash
func ProcessBlock(code WasmCode, block Block, prevVMState VMState, prevManifestReceipt SwarmReceipt,
) (VMState, Manifest, SwarmReceipt, Digest) {
  var vmState = NextVMState(code, prevVMState, block.Txs)
  var txsReceipt = SwarmUpload(pack(block.Txs))

  var manifest = Manifest{
    Header:              block.Header,
    VMStateHash:         MerkleHash(vmState.Memory),
    LastCommit:          block.LastCommit,
    TxsReceipt:          txsReceipt,
    LastManifestReceipt: prevManifestReceipt,
  }
  var receipt = SwarmUpload(pack(manifest))
  var nextAppHash = Hash(pack(manifest))

  return vmState, manifest, receipt, nextAppHash
}

type QueryResponse struct {
  MemoryRegion MemoryRegion // region of the virtual machine memory containing query result
  Proof        MerkleProof  // Merkle Proof for `Memory` belonging to the whole VM memory
  Manifests    [3]Manifest  // block manifests
}

// prepares the query response containing memory region with results
func MakeQueryResponse(manifests [3]Manifest, vmState VMState, offset int32, length int32) QueryResponse {
  var proof = CreateMerkleProof(vmState.Memory, offset, length)
  var memoryRegion = MakeMemoryRegion(vmState.Memory, offset, length)

  return QueryResponse {
    MemoryRegion: memoryRegion, 
    Proof: proof, 
    Manifests: manifests,
  }
}
