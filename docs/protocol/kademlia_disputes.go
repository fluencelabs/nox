package protocol

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
func (contract *SideContract) DisputeSideCheckpoint(startHeight int64, blocks []SideBlock) {
  var startBlock = contract.Checkpoints[startHeight]
  var endBlock = contract.Checkpoints[startHeight + 1]

  // checking that the chain is linked correctly
  for i, block := range blocks {
    var prevBlock SideBlock
    if i == 0 { prevBlock = startBlock } else { prevBlock = blocks[i - 1]}
    if block.PrevBlockHash != Hash(pack(prevBlock)) || (block.Height != prevBlock.Height + 1) {
      // incorrect chain segment, nothing to do here
      return
    }
  }

  if Hash(pack(blocks[len(blocks) - 1])) != Hash(pack(endBlock)) {
    // violation! let's punish offending sidechain nodes!
  }
}
