package fluence.effects.tendermint.block.history.db

// Here's how different data is stored in blockstore.db
/*
func calcBlockMetaKey(height int64) []byte {
	return []byte(fmt.Sprintf("H:%v", height))
}

func calcBlockPartKey(height int64, partIndex int) []byte {
	return []byte(fmt.Sprintf("P:%v:%v", height, partIndex))
}

func calcBlockCommitKey(height int64) []byte {
	return []byte(fmt.Sprintf("C:%v", height))
}

func calcSeenCommitKey(height int64) []byte {
	return []byte(fmt.Sprintf("SC:%v", height))
}
 */

/**
 * Read blocks from blockstore.db which is at
 * `~.fluence/app-89-3/tendermint/data/blockstore.db`
 */
class Blockstore {}
