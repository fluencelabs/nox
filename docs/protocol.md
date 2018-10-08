# Protocol

## Core

Before we start describing the protocol, few words need to be said about the core blocks. Basic cryptographic primitives such as digital signature generation and verification, cryptographic hash computation and Merkle tree composition are listed below and used throughout the rest of the protocol specification. We do not specify exact algorithms such as SHA3, RIPEMD or EdDSA for those primitives but still assume them to behave according to the common expectations.

```go
type MerkleProof struct {
  siblings [][][]byte  // Merkle tree layer –> sibling index in the layer –> sibling (chunk hash)
}

// computes a cryptographic hash of the input data
func Hash(data []byte) []byte {}

// produces a digital signature from the input data using the secret key
func Sign(secretKey []byte, data []byte) []byte {}

// verifies that the digital signature of the input data conforms to the public key
func Verify(publicKey []byte, signature []byte, data []byte) boolean {}

// computes a Merkle root using supplied chunks as leaf data blocks in the Merkle tree
func MerkleRoot(allChunks [][]byte) []byte { }

// generates a Merkle proof for the chunk selected from the chunks list
func CreateMerkleProof(selectedChunk []byte, allChunks [][]byte) MerkleProof {}

// verifies that the Merkle proof of the selected chunk conforms to the Merkle root
func VerifyMerkleProofs(selectedChunk []byte, proof MerkleProof, root []byte) boolean {}
```

## External systems

### Tendermint

Tendermint produces new blocks and feeds them to the state machine. It uses Merkle trees to compute the Merkle hash of certain blocks of data and digital signatures to sign produced blocks, however here we assume these functions are not compatible with Fluence:

```go
func TmHash(data []byte) []byte {}
func TmSign(secretKey []byte, data []byte) []byte {}
func TmVerify(publicKey []byte, signature []byte, data []byte) boolean {}
func TmMerkleRoot(allChunks [][]byte) []byte {}
```

### Ethereum

For the purposes of this protocol Ethereum is treated as a key-value dictionary where a contract can be found by it's address.

### Swarm

Swarm is treated as a hash addressable storage where a content can be found by it's hash. We can assume it exports two functions: `swarm_hash(...)` and `swarm_upload(...)` which calculate the hash of the content and upload the content to Swarm respectively. We assume that Swarm hash function is not compatible neither with Fluence nor with Tendermint. 

We also assume that upload function returns Swarm receipt which indicates that Swarm is fully responsible for the passed content. The receipt contains the Swarm hash of the content and the signature of the Swarm node `S` with the public/private key pair `<S_pk, S_sk>` financially responsible for storing the content. Receipts functionality is not implemented yet in the current Swarm release, however it's described in ["Swap, swear and swindle: incentive system for Swarm"](https://swarm-gateways.net/bzz:/theswarm.eth/ethersphere/orange-papers/1/sw^3.pdf) and can be reasonably expected to show up soon.

```go
type SwarmNode struct {
  PublicKey []byte                    // Swarm node public key
  SecretKey []byte                    // Swarm node secret key
}

type SwarmReceipt struct {
  ContentHash []byte                  // Swarm hash of the stored content
  Insurance   Insurance               // insurance written for the accepted content
}

type Insurance struct {
  Id        []byte                    // Swarm node identifier
  Signature []byte                    // Swarm node signature
}

func SwarmHash(data []byte) []byte {}
func SwarmSign(secretKey []byte, data []byte) []byte {}
func SwarmVerify(publicKey []byte, signature []byte, data []byte) boolean {}
func SwarmUpload(content []byte) SwarmReceipt {}

// rules
var swarm      map[[]byte]interface{}  // Swarm storage: hash(x) –> x
var swarmNodes map[[]byte]SwarmNode    // Swarm nodes: address –> public / private key pair
var content    []byte                  // stored content
var receipt    SwarmReceipt            // receipt issued for the stored content

swarm[SwarmHash(content)] == content
receipt.ContentHash == SwarmHash(content)
receipt.Insurance.Signature == SwarmSign(
  swarmNodes[receipt.Insurance.Id].SecretKey,  // secret key
  Concat(                                      // data
    receipt.ContentHash,
    receipt.Insurance.Id
  )
)
```

## Initial setup

Clients and real-time nodes generate public/private key pairs and join the cluster through the Ethereum smart contract. Every real-time node `R_i` generates the public/private key pair `<R_pk_i, R_sk_i>`. Every client `C_j` generates the public/private key pair `<C_pk_j, C_sk_j>`. Once the cluster is formed the smart contract contains the corresponding public keys:

```java
Ethereum {
  fluence_contract_address –> fluence_contract 
}

fluence_contract: EthereumContract = {  
  nodes: byte[][] = [
    R_pk_1: byte[],
    ...,
    R_pk_n: byte[]
  ],
  clients: byte[][] = [
    C_pk_1: byte[],
    ...,
    C_pk_m: byte[]
  ]
}
```

## Transaction construction

A transaction always has a specific authoring client and carries all information required to execute a deployed WebAssembly function:

```go
type Client struct {
  PublicKey []byte             // client public key
  SecretKey []byte             // client secret key
}

type Transaction struct {
  Invoke []byte                // function name + arguments + client session + session order
  Stamp  Stamp                 // client stamp of the transaction
}

type Stamp struct {
  Id        []byte             // client identifier
  Signature []byte             // client signature
}

// rules
var clients map[[]byte]Client  // clients: address –> public/private key pair
var tx      Transaction        // transaction formed by the client

tx.Signature == Sign(
  clients[tx.Stamp.Id], // secret key
  Concat(               // data
    tx.Invoke,
    tx.Stamp.Id
  )
)
```

## Transaction submission

Once the client `C_j` has constructed a transaction, it is submitted to one of the real-time nodes `R_i` which checks the received transaction:

```java
assert C_pk_j in fluence_contract.clients
assert verify(C_pk_j, hash(tx.payload), tx.signature)
```

If the transaction passes the check, it's added to the mempool, otherwise it's declined.

**Questions:**
- should the real-time node sign the acceptance or refusal of transaction with `R_sk`?


## Tendermint block formation

Tendermint consensus engine periodically pulls few transactions from the mempool and forms a new block:

```go
type TmNode struct {
  PublicKey []byte            // Tendermint node public key
  SecretKey []byte            // Tendermint node secret key
}

type Block struct {
  Header     Header           // block header
  LastCommit []Vote           // Tendermint nodes votes for the previous block
  Txs        []Transaction    // transactions as sent by clients
}

type Header struct {
  LastBlockHash  []byte       // Merkle root of the previous block header fields 
  LastCommitHash []byte       // Merkle root of the last commit votes
  TxsHash        []byte       // Merkle root of the block transactions
  AppHash        []byte       // application state hash after the previous block
}

type Vote struct {
  Address   []byte            // Tendermint node address
  Signature []byte            // Tendermint node signature of the previous block header
}

// rules
var nodes  map[[]byte]TmNode  // Tendermint nodes: address –> public/private key pair
var blocks []Block            // Tendermint blockchain

blocks[k].Header.LastBlockHash == TmMerkleRoot(blocks[k - 1].Header)
blocks[k].Header.LastCommitHash == TmMerkleRoot(blocks[k].LastCommit)
blocks[k].Header.TxsHash == TmMerkleRoot(blocks[k].Txs)
blocks[k].LastCommit[i].Signature == TmSign(
  nodes[blocks[k].LastCommit[i].Address].SecretKey,  // secret key 
  Concat(                                            // data
    blocks[k].Header.LastBlockHash,
    blocks[k].LastCommit[i].Address
  )                     
)
```

## Block processing

Once the block is passed through Tendermint consensus, it is delivered to the state machine. State machine passes block transactions to the WebAssembly VM causing the latter to change state. 

The virtual machine state is essentially a block of memory split into chunks. These chunks can be used to compute the state hash:
```go
type VMState struct {
  Chunks: []VMChunk     // virtual machine memory chunks
}

type VMChunk {
  Data: []byte          // virtual machine memory chunk bytes
}

// applies block transactions to the virtual machine state to produce the new state
func AdvanceVMState(vmState *VMState, txs []Transaction) VMState {}

// rules
var blocks   []Block    // Tendermint blockchain
var vmStates []VMState  // virtual machine states

vmStates[k + 1] == AdvanceVMState(&vmStates[k], blocks[k].Txs)
```

Once the block was processed by the WebAssembly VM, it has to be stored in Swarm for the future batch validation. Blocks are stored as two separate pieces in Swarm: the block manifest and the transactions list. The manifest contains the Swarm hash of the transactions list, which makes it possible to find transactions by having just the manifest:

```go
type Manifest struct {
  Header                Header        // block header
  LastCommit            []Vote        // Tendermint nodes votes for the previous block
  TxsSwarmHash          []byte        // Swarm hash of the block transactions
  VMStateHash           []byte        // virtual machine state hash after the previous block
  LastManifestSwarmHash []byte        // Swarm hash of the previous manifest
}

// creates a new manifest from the block and the previous block
func CreateManifest(block *Block, prevBlock *Block) Manifest {}

// rules
var blocks    []Block                 // Tendermint blockchain
var vmStates  []VMState               // virtual machine states
var manifests []Manifest              // manifests
var swarm     map[[]byte]interface{}  // Swarm storage: hash(x) –> x

manifests[k].Header == blocks[k].Header
manifests[k].LastCommit == blocks[k].LastCommit
manifests[k].TxsSwarmHash == SwarmHash(blocks[k].Txs)
manifests[k].VMStateHash == MerkleRoot(vmStates[k].Chunks)
manifests[k].LastManifestSwarmHash == SwarmHash(manifests[k - 1])

swarm[SwarmHash(manifests[k])] == manifest[k]
swarm[SwarmHash(blocks[k].Txs)] == blocks[k].Txs
```

Once the block manifest is formed and the virtual machine has advanced to the new state, it becomes possible to compute the new application state hash, which will be used in the next block:

```java
blocks[k + 1].Header.AppHash == Hash(manifests[k])
```

## Results verification

Once the cluster has reached consensus on the block, advanced the virtual machine state, reached consensus on the next couple of blocks and saved related block manifests and transactions into Swarm, the client can query results of the function invocation through the ABCI query API. 

Let's assume that transaction sent by the client was included into the block `k`, in this case the client has to wait until the block `k+2` is formed and the corresponding block manifest is uploaded to Swarm. Once this is done, results returned to the client will look the following:

```go
type QueryResults struct {
  Chunks           map[int]VMChunk  // selected virtual machine state chunks
  ChunksProofs     []MerkleProof    // Merkle proofs: chunks belong to the virtual machine state
  Manifests        [3]Manifest      // block manifests
  ManifestReceipts [3]SwarmReceipt  // Swarm receipts for block manifests
  TxsReceipt       SwarmReceipt     // Swarm receipt for block transactions
}

// rules
var blocks    []Block                // Tendermint blockchain
var vmStates  []VMState              // virtual machine states
var manifests []Manifest             // manifests
var results   QueryResults           // results returned for a transaction in block `k`

results.Chunks[t] == vmStates[k + 1].Chunks[t]
results.ChunksProof == CreateMerkleProofs(results.Chunks, vmStates[k + 1].Chunks)
results.Manifests[p] == manifests[k + p]
results.ManifestReceipts[p] == SwarmUpload(results.Manifest[p])
results.TxsReceipt == SwarmUpload(blocks[k].Txs)
```

The client verifies returned results in a few steps.

1) The client checks that every manifest is stored in Swarm properly. This means that receipt is issued for the correct content hash, the Swarm node signature does sign exactly this hash and that the Swarm node has the security deposit big enough.

```java
assert results.manifest_receipt_k+i.content_hash == swarm_hash(results.manifest_k+i)
assert verify(
  public_key = results.manifest_receipt_k+i.node_pk,
  signature = results.manifest_receipt_k+i.node_signature,
  data = swarm_hash(results.manifest_k+i)
)
assert swarm_contract.nodes[results.manifest_receipt_k+i.node_pk].deposit ≥ X ETH
```

2) The client checks that manifests are linked correctly in Swarm.

```java
assert results.manifest_k+i+1.last_manifest_swarm_hash = swarm_hash(results.manifest_k+i)
```

3) The client checks that manifests are linked correctly through the application hash.

```java
assert results.manifest_k+i+1.header.app_hash = hash(results.manifest_k+i)
```

4) The client checks that nodes have actually signed the corresponding block headers.

```java
assert tm_verify(
  public_key = results.manifest_k+i+1.last_commit[j].pk,
  signature = results.manifest_k+i+1.last_commit[j].signature,
  data = tm_merkle(items(results.manifest_k+i.header))
)
assert fluence_contract.nodes[results.manifest_receipt_k+i.last_commit[j].pk].deposit ≥ X ETH
```

5) The client checks that VM state chunks belong to the state hash.

```java
assert merkle_verify(
  data = results.vm_state_chunks, 
  merkle_proof = results.vm_state_chunks_proof, 
  merkle_root = results.manifest_k.vm_state_hash
)
```

----
\> [Twemoji](https://twemoji.twitter.com/) graphics: Twitter, Inc & others [\[CC-BY 4.0\]]( https://creativecommons.org/licenses/by/4.0/)
