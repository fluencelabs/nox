package protocol

// default chunk size for Merkle Proofs over bytes
const FlChunkSize int32 = 4096

// represents a byte range selected from memory to construct a Merkle Proof over it
// if selected range is not aligned to chunk size, then it's extended to be aligned
type MemoryRegion struct {
  ExtendedRegion []byte // selected region of the memory; extended to be aligned to chunks
  offset int32          // start of the byte range in `Region`
  length int32          // length of the byte range
}

// returns original unextended byte range
func (region MemoryRegion) ByteRange() []byte {
  return region.ExtendedRegion[region.offset : region.offset + region.length]
}

func MakeMemoryRegion(data []byte, offset int32, length int32) MemoryRegion {
  var from = offset - offset % FlChunkSize
  var rangeEnd = offset + length
  var to = rangeEnd - rangeEnd % FlChunkSize
  if (rangeEnd % FlChunkSize) != int32(0) {
    to += FlChunkSize
  }

  return MemoryRegion {
    ExtendedRegion: data[from:to],
    offset: offset,
    length: length,
  }
}