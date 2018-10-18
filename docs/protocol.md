# Protocol

- [Core](#core)
- [External systems](#external-systems)
  - [Ethereum](#ethereum)
  - [Swarm](#swarm)
  - [Kademlia sidechain](#kademlia-sidechain)
- [Initial setup](#initial-setup)
- [Transactions](#transactions)
- [Real-time processing](#real-time-processing)
  - [Transaction validation](#transaction-validation)
  - [Tendermint block formation](#tendermint-block-formation)
  - [Block processing](#block-processing)
- [Client](#client)
  - [Query results](#query-results)
  - [Block progress verification](#block-progress-verification)

## Core

Before we start describing the protocol, few words need to be said about the core blocks. Basic cryptographic primitives such as digital signature generation and verification, cryptographic hash computation and Merkle tree composition are listed below and used throughout the rest of the protocol specification. We do not specify exact algorithms such as SHA3, RIPEMD or EdDSA for those primitives but still assume them to behave according to the common expectations.

```go
type Digest     = [32]byte
type PublicKey  = [32]byte
type PrivateKey = [64]byte
type Signature  = [64]byte

type Chunk = []byte

type MerkleProof struct {
  Path     []int       // path from the Merkle tree root to the selected chunk
  Siblings [][]Digest  // Merkle tree layer –> sibling index in the layer –> sibling (chunk hash)
}

type Seal struct {
  PublicKey PublicKey
  Signature Signature
}

// computes a cryptographic hash of the input data
func Hash(data []byte) Digest {}

// produces a digital signature for the input data digest
func Sign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal {}

// verifies that the input data digest is signed correctly
func Verify(seal Seal, digest Digest) bool {}

// computes a Merkle root using supplied chunks as leaf data blocks in the Merkle tree
func MerkleRoot(allChunks []Chunk) Digest {}

// generates a Merkle proof for the chunk selected from the chunks list
func CreateMerkleProof(index int, selectedChunk Chunk, allChunks []Chunk) MerkleProof {}

// verifies that the Merkle proof of the selected chunk conforms to the Merkle root
func VerifyMerkleProof(selectedChunk Chunk, proof *MerkleProof, root Digest) bool {}

```

## External systems

### Ethereum

For the purposes of this protocol specification, Ethereum is viewed as a secure state storage keeping few related smart contracts. Those smart contracts can be checked by any network participants to, for example, make sure that some node still has a security deposit placed. Below we provide an example of such contract.

```go
type ExampleContract struct {
  Collaterals map[PublicKey]int64  // security deposits: node identifier –> deposit size
}

// verifies that a node has enough deposited funds
func VerifyNodeCollateral(exampleContract *ExampleContract, nodeId PublicKey, minCollateral int64) {
  assert(exampleContract.Collaterals[nodeId] >= minCollateral)
}
```

### Swarm

Swarm is treated as a hash addressable storage where a content can be found by it's hash. Swarm has it's own set of cryptographic primitives which we don't expect to be compatible with Fluence core primitives.

```go
// listed Swarm functions carry the same meaning and arguments as core functions
func SwarmHash(data []byte) Digest {}
func SwarmSign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal {}
func SwarmVerify(seal Seal, digest Digest) bool {}

// data
var swarm map[Digest][]byte  // Swarm storage: hash(x) –> x

// rules
var content []byte           // some content

∀ content:
  assert(bytes.Equal(swarm[SwarmHash(content)], content))
```

We expect that every node serving in the Swarm network has an identifier and a public/private key pair and is registered in the publicly accessible Ethereum smart contract.

```go
type SwarmContract struct {
  Collaterals map[PublicKey]int64  // security deposits: node identifier –> deposit size
}

// data
var swarmContract SwarmContract  // Swarm Ethereum smart contract
```

#### Stored content receipts

We assume that Swarm provides an upload function which returns a Swarm receipt indicating Swarm network accountability for the passed content. The receipt contains the Swarm hash of the content and the signature of the Swarm node which is financially responsible for storing the content. Receipts functionality is not implemented yet in the current Swarm release, however it's described in details in ["Swap, swear and swindle: incentive system for Swarm"](https://swarm-gateways.net/bzz:/theswarm.eth/ethersphere/orange-papers/1/sw^3.pdf) paper and can be reasonably expected to be rolled out soon.

```go
type SwarmReceipt struct {
  ContentHash Digest  // Swarm hash of the stored content
  Insurance   Seal    // insurance written by the Swarm node for the accepted content
}

// uploads the content to the Swarm network, returns a receipt of responsibility
func SwarmUpload(content []byte) SwarmReceipt {}

// data
var content []byte // some content

// rules
∀ content:
  var receipt = SwarmUpload(content)

  assert(receipt.ContentHash == SwarmHash(content))
  assert(SwarmVerify(receipt.Insurance, receipt.ContentHash))
```

#### Mutable resource updates

We also rely on the [mutable resource updates (MRU)](https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates) feature in Swarm. While core Swarm allows to access the stored content by its hash, MRU lets its user to associate the content with a specific key and update it from time to time. If core Swarm can be represented as a hash-addressable storage, MRU is more complex and for our purposes can be treated as a nested dictionary.

```go
type SwarmMeta struct {
  Key        string  // resource key
  Version    int     // resource version
  Content    []byte  // uploaded content
  Permission Seal    // owner signature which authorizes the update
}

// data
var swarmMRU map[string]map[int]SwarmMeta  // MRU storage: key –> version –> content
```

Every key created in the MRU storage has an associated owner and needs to be initialized first. Once the key is initialized, it can be updated only by the owner. Having the MRU storage and the set of owners, we can more formally define the rules these entities are expected to follow.

```go
type SwarmOwners struct  {
  data map[string]PublicKey  // owners: resource key –> owner's public key
}

// initializes and returns a new resource key
func (owners *SwarmOwners) Init(permission Seal) string {}

// returns an owner's public key for the specified resource key
func (owners *SwarmOwners) Owner(resourceKey string) PublicKey {}

// data
var swarmOwners SwarmOwners                // list of resource owners for each key
var swarmMRU map[string]map[int]SwarmMeta  // MRU storage: key –> version –> content

// rules
var key     string                         // some key
var version int                            // some version

∀ key, ∀ version:
  // the content stored for specific key and version should be properly authorized by its owner
  var meta = swarmMRU[key][version]

  assert(meta.Key == key)
  assert(meta.Version == version)
  assert(meta.Permission.PublicKey == swarmOwners.Owner(meta.Key))
  assert(SwarmVerify(meta.Permission, SwarmHash(pack(meta.Key, meta.Version, meta.Content))))
```

We can also expect that Swarm will provide stored receipts functionality for the MRU resources. Here we assume the following behavior: every time the resource changes, Swarm issues a receipt for the updated resource key and version along with the new content and owner's signature.

```go
// updates the resource associated with the specific resource key
func SwarmMRUUpdate(meta *SwarmMeta) SwarmReceipt {}

// data
var meta SwarmMeta  // some mutable content

// rules
∀ meta:
  var receipt = SwarmMRUUpdate(&meta)
  assert(receipt.ContentHash == SwarmHash(pack(meta)))
```

### Kademlia sidechain

In fact, Kademlia is not quite an external system but is an essential foundation of the Fluence network. However, for our protocol we use it as a some kind of a sidechain which periodically checkpoints selected blocks to the root chain – Ethereum. Blocks stored in Kademlia sidechain are really tiny compared to the overall data volumes flowing through the network – it's expected they will contain only Swarm receipts and producers signatures most of the time.

```go
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
```

It is expected that every Kademlia sidechain node stores the tail of the chain starting from the last block checkpointed into the contract. Every node verifies there are no forks or incorrect references in the chain tail – otherwise, a dispute is submitted to the contract and offending block producers lose their deposits. 

Every sidechain node also verifies that a checkpointing is performed correctly – i.e. there is a correct block being uploaded to the contract every period of time. Otherwise, another dispute is submitted to the contract, but this time a sidechain node that has uploaded an incorrect checkpoint block will lose a deposit.

```go
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
```

Now, every sidechain node allows producers to upload a new block to it and returns a signature if the block was accepted.

```go
type SideNode struct {
  Tail []SideBlock
}

// appends the block the chain tail and checks it doesn't violate correctness properties
func (node *SideNode) UploadBlock(block *SideBlock) Seal {}
```

## Initial setup

There are few different actor types in the Fluence network: clients, real-time nodes forming Tendermint clusters and batch validators. Every node has an identifier, a public/private key pair and a security deposit, and is registered in the Fluence smart contract.

```go
type FlnContract struct {
  ClientCollaterals     map[PublicKey]int64  // clients: identifier –> deposit size
  NodesCollaterals      map[PublicKey]int64  // real-time nodes: identifier –> deposit size
  ValidatorsCollaterals map[PublicKey]int64  // batch validators: identifier –> deposit size
}

// data
var flnContract FlnContract  // Fluence Ethereum smart contract
```

## Transactions

A transaction always has a specific authoring client and carries all the information required to execute a deployed WebAssembly function:

```go
type Transaction struct {
  Invoke []byte  // function name & arguments + required metadata
  Seal   Seal    // client signature of the transaction
}
```

## Real-time processing

### Transaction validation

Once the client has constructed a transaction, it is submitted to one of the real-time nodes which checks the received transaction:

```go
// verifies that transaction was originated by the client with enough funds deposited
func VerifyTransaction(flnContract *FlnContract, tx *Transaction, minCollateral int64){
  // checking that the client actually exists in the contract
  collateral, ok := flnContract.ClientCollaterals[tx.Seal.PublicKey]
  assert(ok)

  // checking that the client has enough funds
  assert(collateral >= minCollateral)

  // checking that the transaction is signed by this client
  assert(Verify(tx.Seal, Hash(tx.Invoke)))
}
```

If the transaction passes the check, it's added to the mempool and might be later used in forming a block. Otherwise the transaction is declined.

**Open questions:**
- should the real-time node sign an acceptance or refusal of the transaction?
- how the real-time node should check the client's security deposit?


### Tendermint block formation

Tendermint consensus engine produces new blocks filled with client supplied transactions and feeds them to the Fluence state machine. Tendermint uses Merkle trees to compute the Merkle root of certain pieces of data and digital signatures to sign produced blocks, however here we assume these functions are not necessary compatible with Fluence and denote them separately.

```go
// listed Tendermint functions carry the same meaning and arguments as core functions 
func TmHash(data []byte) Digest { panic("") }
func TmSign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal { panic("") }
func TmVerify(seal Seal, digest Digest) bool { panic("") }
func TmMerkleRoot(allChunks []Chunk) Digest { panic("") }
```

Tendermint periodically pulls few transactions from the mempool and forms a new block. Nodes participating in consensus sign produced blocks, however their signatures for a specific block are available only as a part of the next block.

```go
type Block struct {
  Header     Header         // block header
  LastCommit []Seal         // Tendermint nodes votes for the previous block
  Txs        []Transaction  // transactions as sent by clients
}

type Header struct {
  LastBlockHash  Digest  // Merkle root of the previous block header fields
  LastCommitHash Digest  // Merkle root of the last commit votes
  TxsHash        Digest  // Merkle root of the block transactions
  AppHash        Digest  // application state hash after the previous block
}

// data
var blocks      []Block  // Tendermint blockchain

// rules
var k int64              // some block number
var i int                // some Tendermint node index

∀ k:
  assert(blocks[k].Header.LastBlockHash == TmMerkleRoot(packMulti(blocks[k - 1].Header)))
  assert(blocks[k].Header.LastCommitHash == TmMerkleRoot(packMulti(blocks[k].LastCommit)))
  assert(blocks[k].Header.TxsHash == TmMerkleRoot(packMulti(blocks[k].Txs)))
  
  ∀ i:
    assert(TmVerify(blocks[k].LastCommit[i], blocks[k].Header.LastBlockHash))
```

Note we haven't specified here how the application state hash (`Header.AppHash`) is getting calculated – this will be described in the next section.

### Block processing

Once the block has passed through Tendermint consensus, it is delivered to the state machine. State machine passes block transactions to the WebAssembly VM causing the latter to change state. The virtual machine state is essentially a block of memory split into chunks which can be used to compute the virtual machine state hash. VM state `k + 1` arises after processing transactions of the block `k`.

```go
type VMState struct {
  Chunks []Chunk     // virtual machine memory chunks
}

// applies block transactions to the virtual machine state to produce the new state
func NextVMState(vmState *VMState, txs []Transaction) VMState { panic("") }
  
// data
var blocks   []Block    // Tendermint blockchain
var vmStates []VMState  // virtual machine states

// rules
var k int               // some block number

∀ k:
  assert(reflect.DeepEqual(vmStates[k + 1], NextVMState(&vmStates[k], blocks[k].Txs)))
```

Once the block is processed by the WebAssembly VM, it has to be stored in Swarm for the future batch validation. Blocks are stored in two separate pieces in Swarm: the block manifest and the transactions list. The manifest contains the Swarm hash of the transactions list, which makes it possible to find transactions by having just the manifest.

```go
type Manifest struct {
  Header                Header        // block header
  LastCommit            []Seal        // Tendermint nodes signatures for the previous block
  TxsSwarmHash          Digest        // Swarm hash of the block transactions
  VMStateHash           Digest        // virtual machine state hash after the previous block
  LastManifestSwarmHash Digest        // Swarm hash of the previous manifest
}

// creates a new manifest from the block and the previous block
func CreateManifest(block *Block, prevBlock *Block) Manifest { panic("") }

// data
var blocks    []Block            // Tendermint blockchain
var vmStates  []VMState          // virtual machine states
var manifests []Manifest         // manifests
var swarm     map[Digest][]byte  // Swarm storage: hash(x) –> x

// rules
var k int                        // some block number

∀ k:
  assert(manifests[k].Header == blocks[k].Header)
  assert(reflect.DeepEqual(manifests[k].LastCommit, blocks[k].LastCommit))
  assert(manifests[k].TxsSwarmHash == SwarmHash(pack(blocks[k].Txs)))
  assert(manifests[k].VMStateHash == MerkleRoot(vmStates[k].Chunks))
  assert(manifests[k].LastManifestSwarmHash == SwarmHash(pack(manifests[k - 1])))

  assert(bytes.Equal(swarm[SwarmHash(pack(manifests[k]))], pack(manifests[k])))
  assert(bytes.Equal(swarm[SwarmHash(pack(blocks[k].Txs))], pack(blocks[k].Txs)))
```

Now, once the block manifest is formed and the virtual machine has advanced to the new state, it becomes possible to compute the new application state hash, which will be used in the next block.

```go
// data
var blocks    []Block            // Tendermint blockchain
var manifests []Manifest         // manifests

// rules
var k int                        // some block number

∀ k:
  assert(blocks[k + 1].Header.AppHash == Hash(pack(manifests[k])))
```

## Client

### Query results

Once the cluster has reached consensus on the block, advanced the virtual machine state, reached consensus on the next couple of blocks and saved related block manifests and transactions into Swarm, the client can query results of the function invocation through the ABCI query API. 

Let's assume that transaction sent by the client was included into the block `k`. In this case the client has to wait until the block `k + 2` is formed and the corresponding block manifest is uploaded to Swarm. Once this is done, results returned to the client will look the following.

```go
type QueryResults struct {
  Chunks           map[int]Chunk    // selected virtual machine state chunks
  ChunksProofs     []MerkleProof    // Merkle proofs: chunks belong to the virtual machine state
  Manifests        [3]Manifest      // block manifests
  ManifestReceipts [3]SwarmReceipt  // Swarm receipts for block manifests
  TxsReceipt       SwarmReceipt     // Swarm receipt for block transactions
}

  // data
  var blocks         []Block        // Tendermint blockchain
  var vmStates       []VMState      // virtual machine states
  var manifests      []Manifest     // manifests for blocks stored in Swarm

  // rules
  var k       int                   // some block number
  var t       int                   // some virtual machine state chunk number
  var p       int                   // some manifest index

  var results QueryResults          // results returned for the block `k`

  ∀ k:
    ∀ t ∈ range results.Chunks:
      assert(bytes.Equal(results.Chunks[t], vmStates[k + 1].Chunks[t]))
      assert(reflect.DeepEqual(results.ChunksProofs[t], CreateMerkleProof(t, results.Chunks[t], vmStates[k + 1].Chunks)))

    ∀ p ∈ [0, 3):
      assert(reflect.DeepEqual(results.Manifests[p], manifests[k + p]))
      assert(results.ManifestReceipts[p] == SwarmUpload(pack(results.Manifests[p])))

      assert(results.TxsReceipt == SwarmUpload(pack(blocks[k].Txs)))
```

### Block progress verification

The client verifies that returned results represent correct block progress in a few steps. Below we will list those steps, but first we need to mention that they are not verifying that the transaction sent by the client was actually processed.

Instead, all the client does verify here is that the virtual machine state progress made by executing the block `k` was saved properly in Swarm for the future batch validation. In this case, if the state transition was performed incorrectly, real-time nodes deposits will be slashed.

However, an all-malicious cluster might never include the transaction sent by the client. In this case the new virtual machine state won't have the corresponding function return value. It might also happen that a malicious cluster will include an invalid transaction into the block. For example, that might be a transaction that was never originated by the correct client but still changing the state.

These aspects will be considered in another section, and for now we will focus on how the block progress is being verified.

#### Verification of manifests Swarm storage

The client checks that every manifest is stored in Swarm properly. This means that receipt is issued for the correct content hash, the Swarm node signature does sign exactly this hash and that the Swarm node has the security deposit big enough.

```go
func VerifyResultsManifestsStorage(results QueryResults, minCollateral int) {
  for p := 0; p < 3; p++ {
    var swarmNodeId = results.ManifestReceipts[p].Insurance.NodeId
    
    // checking that the receipt is issued for the correct manifest
    assert(results.ManifestReceipts[p].ContentHash == SwarmHash(results.Manifest[p]))
    
    // checking that the swarm node has enough funds
    assert(swarmContract[swarmNodeId].Collateral >= minCollateral)
    
    // checking that the receipt is signed by this swarm node
    assert(
      SwarmVerify(
        swarmContract[swarmNodeId].PublicKey,             // public key
        results.ManifestReceipts[p].Insurance.Signature,  // signature
        results.ManifestReceipts[p].ContentHash           // data
      )
    )      
  }
}
```

#### Verification of manifests Swarm connectivity

The client checks that manifests are linked correctly in Swarm.

```go
func VerifyResultsSwarmConnectivity(results QueryResults) {
  for p := 0; p < 2; p++ {
    assert(results.Manifest[p + 1].LastManifestSwarmHash == SwarmHash(results.Manifest[p]))
  }
}
```

#### Verification of manifests application state connectivity 
The client checks that manifests are linked correctly through the application state hash.

```go
func VerifyResultsAppStateConnectivity(results QueryResults) {
  for p := 0; p < 2; p++ {
    assert(results.Manifest[p + 1].Header.AppHash == Hash(results.Manifest[p]))
  }
}
```

#### Verification of blocks correctness

The client checks that BFT consensus was reached on the blocks propagation, real-time nodes have actually signed the corresponding block headers and that each node has at least the minimal collateral posted in the Fluence smart contract.

```go
func VerifyResultsBlocks(results QueryResults) {
  for p := 0; p < 2; p++ {
    // checking that BFT consensus was actually reached
    assert(len(results.Manifest[p + 1].LastCommit) > float64(2/3) * len(flnContract.nodes))
    
    for _, vote := range results.Manifest[p + 1].LastCommit {
      var tmNodeId = vote.Address
      
      // checking that the real-time node has enough funds
      assert(flnContract.nodes[tmNodeId].Collateral >= minTmDeposit)

      // checking that the block commit is signed by this node
      assert(
        TmVerify(
          flnContract.Nodes[tmNodeId].PublicKey,            // public key
          results.Manifest[p + 1].LastCommit[i].Signature,  // signature
          TmMerkle(results.Manifest[p].Header)              // data
        )    
      )      
    }
  }
}
```

#### Verification of Merkle proofs for returned VM state chunks

The client checks that returned virtual machine state chunks belong to the virtual machine state hash.

```go
func VerifyResultsChunks(results QueryResults) {
  for t := range results.Chunks {
    assert(
      VerifyMerkleProof(
        results.Chunks[t],                // selected chunk
        results.ChunksProofs[t],          // Merkle proof
        results.Manifests[1].VMStateHash  // Merkle root
      )
    )
  }    
}
```