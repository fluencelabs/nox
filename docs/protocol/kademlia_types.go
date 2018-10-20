package protocol

type SideBlock struct {
  Height        int64   // block height
  PrevBlockHash Digest  // hash of the previous block
  Data          []byte  // block data
  Signatures    []Seal  // signatures of the block producers
}

type SideContract struct {
  CheckpointInterval int                  // how often the blocks should be checkpointed
  Checkpoints        map[int64]SideBlock  // checkpoints: block height –> block
}
