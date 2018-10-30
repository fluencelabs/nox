package protocol

type SideBlock struct {
	Height        int64        // side block height
	PrevBlockHash Digest       // hash of the previous side block
	Receipt       SwarmReceipt // Swarm receipt for the content associated with the side block
	Signatures    []Seal       // signatures of the side block producers
}

type SideFluenceContract struct {
	base              BasicFluenceContract // parent contract
	SideNodesDeposits map[PublicKey]int64  // sidechain nodes: identifier –> deposit size

	CheckpointInterval    int64               // how often blocks should be checkpointed
	Checkpoints           []SideBlock         // block checkpoints
	CheckpointsByHeight   map[int64]SideBlock // block height –> block
	CheckpointsSignatures map[int64][]Seal    // block height –> sidechain nodes signatures
}

// punishes block producers if blocks are not linked correctly
func (contract SideFluenceContract) DisputeReference(block SideBlock, nextBlock SideBlock) {
	if nextBlock.PrevBlockHash != Hash(pack(block)) && nextBlock.Height == block.Height+1 {

		// violation! let's punish producers signed the next block!
		for _, seal := range nextBlock.Signatures {
			contract.base.NodesDeposits[seal.PublicKey] = 0
		}
	}
}

// punishes block producers if a fork is present
func (contract SideFluenceContract) DisputeFork(block1 SideBlock, block2 SideBlock) {
	if block1.PrevBlockHash == block2.PrevBlockHash {

		// violation! let's punish producers signed both blocks!
		var m = make(map[PublicKey]bool)
		for _, seal := range block1.Signatures {
			m[seal.PublicKey] = true
		}

		for _, seal := range block2.Signatures {
			if m[seal.PublicKey] {
				contract.base.NodesDeposits[seal.PublicKey] = 0
			}
		}
	}
}

// punishes sidechain nodes if the block is checkpointed incorrectly
func (contract SideFluenceContract) DisputeCheckpoint(index int, blocks []SideBlock) {
	var prevCheckpoint = contract.Checkpoints[index-1]
	var checkpoint = contract.Checkpoints[index]

	// checking that the chain is linked correctly
	for i, block := range blocks {
		var prevBlock SideBlock
		if i == 0 {
			prevBlock = prevCheckpoint
		} else {
			prevBlock = blocks[i-1]
		}
		if block.PrevBlockHash != Hash(pack(prevBlock)) || (block.Height != prevBlock.Height+1) {
			// incorrect subchain, nothing to do here
			return
		}
	}

	if Hash(pack(blocks[len(blocks)-1])) != Hash(pack(checkpoint)) {
		// violation! let's punish sidechain nodes uploaded the checkpoint!
		for _, seal := range contract.CheckpointsSignatures[checkpoint.Height] {
			contract.SideNodesDeposits[seal.PublicKey] = 0
		}
	}
}

type SideNode struct {
	Tail []SideBlock
}

// appends the block to the chain tail and checks it doesn't violate correctness properties
func (node SideNode) UploadBlock(block SideBlock) Seal { panic("") }
