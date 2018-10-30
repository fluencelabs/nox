package protocol

// verifies that a transaction was originated by the client with enough funds deposited
func VerifyTransaction(contract BasicFluenceContract, tx Transaction, minDeposit int64) {
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
func TmVerify(seal Seal, digest Digest) bool                                { panic("") }
func TmMerkleRoot(chunks []Chunk) Digest                                    { panic("") }

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

type TmNode struct {
	PublicKey  PublicKey  // real-time node public key
	privateKey PrivateKey // real-time node private key
}

// signs the block assuming the node has voted for it during consensus settlement
func (node TmNode) SignBlockHash(blockHash Digest) Seal {
	return TmSign(node.PublicKey, node.privateKey, blockHash)
}

// prepares the block (assuming the nodes have reached a consensus)
func PrepareBlock(nodes []TmNode, prevBlock Block, txs Transactions, appHash Digest) Block {
	var lastBlockHash = TmMerkleRoot(packMulti(prevBlock.Header))
	var lastCommit = make([]Seal, 0, len(nodes))
	for i, node := range nodes {
		lastCommit[i] = node.SignBlockHash(lastBlockHash)
	}

	return Block{
		Header: Header{
			LastBlockHash:  lastBlockHash,
			LastCommitHash: TmMerkleRoot(packMulti(lastCommit)),
			TxsHash:        TmMerkleRoot(packMulti(txs)),
			AppHash:        appHash,
		},
		LastCommit: lastCommit,
		Txs:        txs,
	}
}

type VMState struct {
	Chunks []Chunk // virtual machine memory chunks
}

// deserializes a byte array into the virtual machine state
func VMStateUnpack([]byte) VMState { panic("") }

// applies block transactions to the virtual machine state to produce the new state
func NextVMState(vmState VMState, txs []Transaction) VMState { panic("") }

type Manifest struct {
	Header              Header       // block header
	VMStateHash         Digest       // hash of the VM state derived by applying the block
	LastCommit          []Seal       // Tendermint nodes signatures for the previous block header
	TxsReceipt          SwarmReceipt // Swarm hash of the block transactions
	LastManifestReceipt SwarmReceipt // Swarm hash of the previous manifest
}

// deserializes a byte array into the manifest
func ManifestUnpack([]byte) Manifest { panic("") }

// returns the new virtual machine state, the manifest for the stored block and the next app hash
func ProcessBlock(block Block, prevVMState VMState, prevManifestReceipt SwarmReceipt,
) (VMState, Manifest, SwarmReceipt, Digest) {
	var vmState = NextVMState(prevVMState, block.Txs)
	var txsReceipt = SwarmUpload(pack(block.Txs))

	var manifest = Manifest{
		Header:              block.Header,
		VMStateHash:         MerkleRoot(vmState.Chunks),
		LastCommit:          block.LastCommit,
		TxsReceipt:          txsReceipt,
		LastManifestReceipt: prevManifestReceipt,
	}
	var receipt = SwarmUpload(pack(manifest))
	var nextAppHash = Hash(pack(manifest))

	return vmState, manifest, receipt, nextAppHash
}

type QueryResponse struct {
	Chunks    map[int]Chunk       // selected virtual machine state chunks
	Proofs    map[int]MerkleProof // Merkle proofs: chunks belong to the virtual machine state
	Manifests [3]Manifest         // block manifests
}

// prepares the query response
func MakeQueryResponse(manifests [3]Manifest, vmState VMState, chunksIndices []int) QueryResponse {
	var chunks = make(map[int]Chunk)
	var proofs = make(map[int]MerkleProof)

	for _, index := range chunksIndices {
		var chunk = vmState.Chunks[index]
		chunks[index] = chunk

		proofs[index] = CreateMerkleProof(vmState.Chunks, int32(index))
	}

	return QueryResponse{Chunks: chunks, Proofs: proofs, Manifests: manifests}
}
