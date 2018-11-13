package protocol

// default chunk size for Merkle proofs over bytes
const FlChunkSize uint64 = 4096

// represents a byte range selected from memory to construct a Merkle proof over it
// if selected range boundaries aren't aligned to chunk boundaries, then it's extended to be aligned
type ByteRegion struct {
  AlignedRegion []byte // selected region of the memory; extended to be aligned to chunks
  offset        uint64 // start of the byte range in `AlignedRegion`
  length        uint64 // length of the byte range
}

// returns the original unextended byte range
func (region ByteRegion) Data() []byte {
  return region.AlignedRegion[region.offset : region.offset+region.length]
}

// creates a ByteRegion instance containing an extended byte range
func MakeByteRegion(data []byte, offset uint64, length uint64) ByteRegion {
  var relativeOffset = offset % FlChunkSize 
  var from = offset - relativeOffset
  var rangeEnd = offset + length
  var to = rangeEnd - rangeEnd % FlChunkSize
  if (rangeEnd % FlChunkSize) != uint64(0) {
    to += FlChunkSize
  }

  return ByteRegion{
    AlignedRegion: data[from:to],
    offset:        relativeOffset,
    length:        length,
  }
}
