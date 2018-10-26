package protocol

// todo: check block producers
// todo: store sidechain nodes signatures in the contract

type SideBlock struct {
  Height        int64         // side block height
  PrevBlockHash Digest        // hash of the previous side block
  Receipt       SwarmReceipt  // Swarm receipt for the content associated with the side block
  Signatures    []Seal        // signatures of the side block producers
}

type SideContract struct {
  CheckpointInterval  int64               // how often blocks should be checkpointed
  Checkpoints         []SideBlock         // block checkpoints
  CheckpointsByHeight map[int64]SideBlock // block height –> block
}

// punishes block producers if blocks are not linked correctly
func (contract *SideContract) DisputeSideReference(block SideBlock, nextBlock SideBlock) {
  if nextBlock.PrevBlockHash != Hash(pack(block)) && nextBlock.Height == block.Height + 1 {
    // violation! let's punish offending producers!
  }
}

// punishes block producers if a fork is present
func (contract *SideContract) DisputeSideFork(block1 SideBlock, block2 SideBlock) {
  if block1.PrevBlockHash == block2.PrevBlockHash {
    // violation! let's punish offending producers!
  }
}

// punishes sidechain nodes if the block is checkpointed incorrectly
func (contract *SideContract) DisputeSideCheckpoint(index int, blocks []SideBlock) {
  var prevCheckpoint = contract.Checkpoints[index - 1]
  var checkpoint = contract.Checkpoints[index]

  // checking that the chain is linked correctly
  for i, block := range blocks {
    var prevBlock SideBlock
    if i == 0 { prevBlock = prevCheckpoint } else { prevBlock = blocks[i - 1]}
    if block.PrevBlockHash != Hash(pack(prevBlock)) || (block.Height != prevBlock.Height + 1) {
      // incorrect chain segment, nothing to do here
      return
    }
  }

  if Hash(pack(blocks[len(blocks) - 1])) != Hash(pack(checkpoint)) {
    // violation! let's punish offending sidechain nodes!
  }
}

type SideNode struct {
  Tail []SideBlock
}

// appends the block the chain tail and checks it doesn't violate correctness properties
func (node *SideNode) UploadBlock(block SideBlock) Seal { panic("") }
