### Range Merkle proofs

This protocol uses a variation of the Merkle proof algorithm allowing to verify not only an inclusion of the data block into the original byte sequence, but also its index in the original sequence. Furthermore, it allows to construct a single proof for a contiguous range of data blocks.

To construct the Merkle proof, a sequence of bytes first has to be split into multiple data blocks – chunks. Chunk size might vary, but by default `4KB` chunk size is used.

```go
// splits the byte sequence into chunks of specific size
func Split(data []byte, chunkSize int32) []Chunk {}
```

Once the list of chunks is produced, a typical Merkle tree can be build on top of it. On the diagram below we can use a binary encoding of the tree nodes: on each layer <code>L<sub>k</sub></code> the node would have an index <code>a<sub>1</sub>...a<sub>k</sub></code> where <code>a<sub>k</sub></code> equals `0` if it's the left sibling and `1` if it's the right one.

<p align="center">
  <img src="images/range_merkle_tree.png" alt="Range Merkle Tree" width="871px"/>
</p>

The Merkle proof itself is constructed recursively by layers. First, a hash function is applied to every chunk on the layer `C` generating the layer <code>L<sub>n</sub></code> where `n` is the tree depth. Now, starting from the layer <code>L<sub>n</sub></code> we iterate to the layer <code>L<sub>0</sub></code> in the following way. 

_Expand._ On the layer <code>L<sub>n</sub></code> we have a contiguous sequence of hashes having indices ranging from <code>a<sub>1</sub>...a<sub>n–1</sub>a<sub>n</sub></code> to <code>b<sub>1</sub>...b<sub>n–1</sub>b<sub>n</sub></code>. We expand this sequence to contain hashes from <code>a<sub>1</sub>...a<sub>n–1</sub>0</code> to <code>b<sub>1</sub>...b<sub>n–1</sub>1</code>. It's obvious to see that at most two hashes are added on the left and on the right side – these hashes will be sent as a part of the Merkle proof. 

_Lift._ Combining hashes pairwise and applying the hash function we can compute hashes for the next level: a pair <code>(x<sub>1</sub>...x<sub>n–1</sub>0, x<sub>1</sub>...x<sub>n–1</sub>1)</code> on the level <code>L<sub>n</sub></code> produces the hash <code>x<sub>1</sub>...x<sub>n–1</sub></code> on the level <code>L<sub>n–1</sub></code>. It should be clear that for every index <code>x<sub>1</sub>...x<sub>n–1</sub></code> such that <code>a<sub>1</sub>...a<sub>n–1</sub> ≤  x<sub>1</sub>...x<sub>n–1</sub> ≤ b<sub>1</sub>...b<sub>n–1</sub></code> the aforementioned pair would belong to the extended sequence of hashes <code>[a<sub>1</sub>...a<sub>n–1</sub>0;  b<sub>1</sub>...b<sub>n–1</sub>1]</code>.

This construction means we have collected a sequence of hashes <code>[a<sub>1</sub>...a<sub>n–1</sub>;  b<sub>1</sub>...b<sub>n–1</sub>]</code> for the layer <code>L<sub>n–1</sub></code>. Now we can repeat expansion and lift procedures for the next layer until the Merkle root is reached. Note that if the selected chunks range contains just a single chunk, this algorithm virtually generates an ordinary Merkle Proof.

On each layer we have added at most two hashes, so the final Merkle proof looks the following:

```go
type MerkleProofLayer struct {
  left  *Digest
  right *Digest
}

type RangeMerkleProof struct {
  Layers []MerkleProofLayer
}
```

For our example tree, the proof is presented below.

<p align="center">
  <img src="images/range_merkle_proof.png" alt="Range Merkle Proof" width="382px"/>
</p>

Proof verification happens in the way similar to how it was constructed. The verifier starts with the lowest level <code>L<sub>n</sub></code> and goes upward by combining already known hashes with hashes supplied in the Merkle proof. If eventually the already known Merkle root is produced, the proof is deemed correct. 

Note that if for the layer <code>L<sub>k</sub></code> we use `0` as <code>p<sub>k</sub></code> if the left extension hash is not present and `1` if it is, resulting <code>p<sub>1</sub>...p<sub>k</sub></code> index will be equal to the start chunk index in the chunks range. That happens because in our construction we were expanding the sequence of hashes to the left if and only if the left most hash in the sequence had the index of form <code>a<sub>1</sub>...a<sub>k–1</sub>1</code>. 

This means the verifier is able to compute the start and stop chunk indices positions based on the supplied proof.