package protocol

// default chunk size for Merkle Proofs over bytes
const FlChunkSize uint64 = 4096

// represents a byte range selected from memory to construct a Merkle Proof over it
// if selected range is not aligned to chunk size, then it's extended to be aligned
type ByteRegion struct {
  ExtendedRegion []byte // selected region of the memory; extended to be aligned to chunks
  offset uint64          // start of the byte range in `Region`
  length uint64          // length of the byte range
}

// returns the original unextended byte range
func (region ByteRegion) Data() []byte {
  return region.ExtendedRegion[region.offset : region.offset + region.length]
}

func MakeByteRegion(data []byte, offset uint64, length uint64) ByteRegion {
  var from = offset - offset % FlChunkSize
  var rangeEnd = offset + length
  var to = rangeEnd - rangeEnd % FlChunkSize
  if (rangeEnd % FlChunkSize) != uint64(0) {
    to += FlChunkSize
  }

  return ByteRegion {
    ExtendedRegion: data[from:to],
    offset: offset,
    length: length,
  }
}