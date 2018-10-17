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
type MerkleProof struct {
  Siblings  [][][]byte  // Merkle tree layer –> sibling index in the layer –> sibling (chunk hash)
}

type Seal struct {
  PublicKey []byte  // signer public key
  Signature []byte  // signer signature
}

// computes a cryptographic hash of the input data
func Hash(data []byte) []byte {}

// produces a digital signature from the input data using the private key
func Sign(publicKey []byte, privateKey []byte, data []byte) Seal {}

// verifies that the digital signature of the input data conforms to the public key
func Verify(publicKey []byte, signature []byte, data []byte) boolean {}

// computes a Merkle root using supplied chunks as leaf data blocks in the Merkle tree
func MerkleRoot(allChunks [][]byte) []byte {}

// generates a Merkle proof for the chunk selected from the chunks list
func CreateMerkleProof(index int, selectedChunk []byte, allChunks [][]byte) MerkleProof {}

// verifies that the Merkle proof of the selected chunk conforms to the Merkle root
func VerifyMerkleProof(selectedChunk []byte, proof *MerkleProof, root []byte) boolean {}
```

## External systems

### Ethereum

For the purposes of this protocol specification, Ethereum is viewed as a secure state storage keeping few related smart contracts. Those smart contracts can be checked by any network participants to, for example, make sure that some node still has a security deposit placed. Below we provide an example of such contract.

```go
type ExampleContract struct {
  Collaterals map[[]byte]int64       // security deposits: node identifier –> deposit size
}

// data
var exampleContract ExampleContract  // example contract instance

// verification
func VerifyNodeCollateral(nodeId []byte, minCollateral int64) {
  assert(exampleContract.Collaterals[nodeId] >= minCollateral)
}
```

### Swarm

Swarm is treated as a hash addressable storage where a content can be found by it's hash. Swarm has it's own set of cryptographic primitives which we don't expect to be compatible with Fluence core primitives.

```go
// listed Swarm functions carry the same meaning and arguments as core functions 
func SwarmHash(data []byte) []byte {}
func SwarmSign(publicKey []byte, privateKey []byte, data []byte) Seal {}
func SwarmVerify(publicKey []byte, signature []byte, data []byte) boolean {}

// data
var swarm map[[]byte]interface{}  // Swarm storage: hash(x) –> x

// rules
var content []byte                // some content

∀ content:
  assert(swarm[SwarmHash(content)] == content)
```

We expect that every node serving in the Swarm network has an identifier and a public/private key pair and is registered in the publicly accessible Ethereum smart contract.

```go
type SwarmContract struct {
  Nodes map[[]byte]SwarmNode     // Swarm nodes: address –> node
}

type SwarmNode struct {
  PublicKey  []byte              // Swarm node public key
  PrivateKey []byte              // Swarm node private key
  Collateral int64               // Swarm node security deposit
}

// data
var swarmContract SwarmContract  // Swarm Ethereum smart contract
```

#### Stored content receipts

We assume that Swarm provides an upload function which returns a Swarm receipt indicating Swarm network accountability for the passed content. The receipt contains the Swarm hash of the content and the signature of the Swarm node which is financially responsible for storing the content. Receipts functionality is not implemented yet in the current Swarm release, however it's described in details in ["Swap, swear and swindle: incentive system for Swarm"](https://swarm-gateways.net/bzz:/theswarm.eth/ethersphere/orange-papers/1/sw^3.pdf) paper and can be reasonably expected to be rolled out soon.

```go
type SwarmReceipt struct {
  ContentHash []byte             // Swarm hash of the stored content
  Insurance   Seal               // insurance written for the accepted content
}

// uploads the content to the Swarm network, returns a receipt of responsibility
func SwarmUpload(content []byte) SwarmReceipt {}

// data
var swarmContract SwarmContract  // Swarm Ethereum smart contract

// rules
var content []byte               // some content

∀ content:
  var receipt = SwarmUpload(content)

  assert(receipt.ContentHash == SwarmHash(content))
  assert(
    receipt.Insurance.Signature == SwarmSign(
      swarmContract.Nodes[receipt.Insurance.NodeId].PublicKey,   // public key
      swarmContract.Nodes[receipt.Insurance.NodeId].PrivateKey,  // private key
      receipt.ContentHash                                        // data
    )
  )
```

#### Mutable resource updates

We also rely on the [mutable resource updates (MRU)](https://swarm-guide.readthedocs.io/en/latest/usage.html#mutable-resource-updates) feature in Swarm. While core Swarm allows to access the stored content by its hash, MRU lets its user to associate the content with a specific key and update it from time to time. If core Swarm can be represented as a hash-addressable storage, MRU is more complex and for our purposes can be treated as a nested dictionary.

```go
type SwarmMeta struct {
  Key       string                         // resource key
  Version   int                            // resource version
  Content   []byte                         // uploaded content
  Signature []byte                         // owner signature which authorizes the update
}

// data
var swarmMRU map[string]map[int]SwarmMeta  // MRU storage: key –> version –> content
```

Every key created in the MRU storage has an associated owner and needs to be initialized first. Once the key is initialized, it can be updated only by the owner.

```go
type SwarmOwners struct {
  data map[string][]byte    // owners: resource key –> public key
}

// initializes and returns a new resource key associated with the passed owner's public key
func (owners *SwarmOwners) Init(publicKey []byte, signature []byte) string {}

// returns an owner's public key for the specified resource key
func (owners *SwarmOwners) Owner(resourceKey string) []byte {}

// data
var swarmOwners SwarmOwners  // list of resource owners
```

Now, having the MRU storage and the set of owners, we can more formally define the rules these entities are expected to follow.


```go
// data
var swarmOwners SwarmOwners                // list of resource owners
var swarmMRU map[string]map[int]SwarmMeta  // MRU storage: key –> version –> content

// rules
var key     string                         // some key
var version int                            // some version

∀ key, ∀ version:
  // the content stored for specific key and version should be authorized by its owner
  var meta = swarmMRU[key][version]
  
  assert(meta.Key == key)
  assert(meta.Version == version)
  
  assert(
    SwarmVerify(
      swarmOwners.Owner(meta.Key),                       // public key
      meta.Signature,                                    // signature
      SwarmHash([meta.Key, meta.Version, meta.Content])  // data
    )
  )
```

We can also expect that Swarm will provide stored receipts functionality for the MRU resources. Here we assume the following behavior: every time the resource changes, Swarm issues a receipt for the updated resource key and version along with the new content and owner's signature.

```go
// updates the resource associated with the specific resource key
func SwarmMRUUpdate(meta *SwarmMeta) SwarmReceipt {}

// rules
var meta SwarmMeta  // some mutable content

∀ meta:
  var receipt = SwarmMRUUpdate(&meta)
  
  assert(receipt.ContentHash == SwarmHash(meta))
```

### Kademlia sidechain

In fact, Kademlia is not quite an external system but is an essential foundation of the Fluence network. However, for our protocol we use it as a some kind of a sidechain which periodically checkpoints selected blocks to the root chain – Ethereum. Blocks stored in Kademlia sidechain are really tiny compared to the overall data volumes flowing through the network – it's expected they will contain only Swarm receipts and producers signatures most of the time.

```go
type SideBlock struct {
  Height        int64                     // block height
  PrevBlockHash []byte                    // hash of the previous block
  Data          []byte                    // block data
  Signatures    []Seal                    // signatures of the block producers  
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
  if nextBlock.PrevBlockHash != Hash(block) && nextBlock.Height == block.Height + 1 {
    // violation! let's punish offending producers!
    ...
  }
}

// punishes block producers if a fork is present
func (contract *SideContract) DisputeSideFork(block1 SideBlock, block2 SideBlock) {
  if block1.PrevBlockHash == block2.PrevBlockHash {
    // violation! let's punish offending producers!
    ...
  }
}

// punishes sidechain nodes if the block is checkpointed incorrectly
func (contract *SideContract) DisputeSideCheckpoint(startHeight int, blocks []SideBlock) {
  var startBlock = contract.Checkpoints[startHeight]
  var endBlock = contract.Checkpoints[startHeight + 1]
    
  // checking that the chain is linked correctly
  for i, block := range blocks {
    var prevBlock SideBlock
    if (i == 0) { prevBlock = startBlock } else { prevBlock = blocks[i - 1]}      
    if block.PrevBlockHash != Hash(prevBlock) || block.Height != prevBlock.Height + 1) {
      // incorrect chain segment, nothing to do here
      return
    }
  }
  
  if Hash(blocks[len(blocks) - 1]) != Hash(endBlock) {
    // violation! let's punish offending sidechain nodes!
    ...
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
  Clients    map[[]byte]Client          // clients: address –> client
  Nodes      map[[]byte]TmNode          // Tendermint nodes: address –> node
  Validators map[[]byte]BatchValidator  // batch validators: address –> validator
}

type Client struct {
  PublicKey  []byte                     // client public key
  PrivateKey []byte                     // client private key
  Collateral int64                      // client security deposit
}

type TmNode struct {
  PublicKey  []byte                     // Tendermint node public key
  PrivateKey []byte                     // Tendermint node private key
  Collateral int64                      // Tendermint node security deposit
}

type BatchValidator struct {
  PublicKey  []byte                     // batch validator public key
  PrivateKey []byte                     // batch validator private key
  Collateral int64                      // batch validator security deposit
}

// data
var flnContract FlnContract             // Fluence Ethereum smart contract
```

## Transactions

A transaction always has a specific authoring client and carries all the information required to execute a deployed WebAssembly function:

```go
type Transaction struct {
  Invoke []byte               // function name & arguments + required metadata
  Seal   Seal                 // client signature of the transaction
}

// data
var flnContract FlnContract   // Fluence Ethereum smart contract

// rules
var tx Transaction            // some transaction formed by the client

∀ tx:
  assert(
    tx.Seal == Sign(
      flnContract.Clients[tx.Stamp.Id].PublicKey,   // public key
      flnContract.Clients[tx.Stamp.Id].PrivateKey,  // private key
      Hash(tx.Invoke)                               // data
    )
  )
```

## Real-time processing

### Transaction validation

Once the client has constructed a transaction, it is submitted to one of the real-time nodes which checks the received transaction:

```go
// data
var flnContract FlnContract  // Fluence Ethereum smart contract

// verification
def VerifyTransaction(var tx Transaction, minCollateral int){
  // checking that the client actually exists in the contract
  client, ok := flnContract.Clients[tx.Stamp.Id]
  assert(ok)
  
  // checking that the client has enough funds
  assert(client.Collateral >= minCollateral)
  
  // checking that the transaction is signed by this client
  assert(
    Verify(
      client.PublicKey,    // public key
      tx.Stamp.Signature,  // signature 
      Hash(invoke)         // data
    )
  )
}
```

If the transaction passes the check, it's added to the mempool and might be later used in forming a block. Otherwise the transaction is declined.

**Questions:**
- should the real-time node sign an acceptance or refusal of the transaction?
- how the real-time node should check the client's security deposit?


### Tendermint block formation

Tendermint consensus engine produces new blocks filled with client supplied transactions and feeds them to the Fluence state machine. Tendermint uses Merkle trees to compute the Merkle root of certain pieces of data and digital signatures to sign produced blocks, however here we assume these functions are not necessary compatible with Fluence and denote them separately.

```go
// listed Tendermint functions carry the same meaning and arguments as core functions 
func TmHash(data []byte) []byte {}
func TmSign(publicKey []byte, privateKey []byte, data []byte) Seal {}
func TmVerify(publicKey []byte, signature []byte, data []byte) boolean {}
func TmMerkleRoot(allChunks [][]byte) []byte {}
```

Tendermint periodically pulls few transactions from the mempool and forms a new block. Nodes participating in consensus sign produced blocks, however their signatures for a specific block are available only as a part of the next block.

```go
type Block struct {
  Header     Header           // block header
  LastCommit []Seal           // Tendermint nodes votes for the previous block
  Txs        []Transaction    // transactions as sent by clients
}

type Header struct {
  LastBlockHash  []byte       // Merkle root of the previous block header fields 
  LastCommitHash []byte       // Merkle root of the last commit votes
  TxsHash        []byte       // Merkle root of the block transactions
  AppHash        []byte       // application state hash after the previous block
}

// data
var flnContract FlnContract   // Fluence Ethereum smart contract
var blocks      []Block       // Tendermint blockchain

// rules
var k int                     // some block number

∀ k:
  assert(blocks[k].Header.LastBlockHash == TmMerkleRoot(blocks[k - 1].Header))
  assert(blocks[k].Header.LastCommitHash == TmMerkleRoot(blocks[k].LastCommit))
  assert(blocks[k].Header.TxsHash == TmMerkleRoot(blocks[k].Txs))
  assert(
    blocks[k].LastCommit[i].Signature == TmSign(
      flnContract.Nodes[blocks[k].LastCommit[i].Address].PublicKey,   // public key
      flnContract.Nodes[blocks[k].LastCommit[i].Address].PrivateKey,  // private key 
      blocks[k].Header.LastBlockHash                                  // data
    )
  )
```

Note we haven't specified here how the application state hash (`Header.AppHash`) is getting calculated – this will be described in the next section.

### Block processing

Once the block has passed through Tendermint consensus, it is delivered to the state machine. State machine passes block transactions to the WebAssembly VM causing the latter to change state. The virtual machine state is essentially a block of memory split into chunks which can be used to compute the virtual machine state hash. VM state `k + 1` arises after processing transactions of the block `k`.

```go
type VMState struct {
  Chunks: []VMChunk     // virtual machine memory chunks
}

type VMChunk {
  Data: []byte          // virtual machine memory chunk bytes
}

// applies block transactions to the virtual machine state to produce the new state
func NextVMState(vmState *VMState, txs []Transaction) VMState {}

// data
var blocks   []Block    // Tendermint blockchain
var vmStates []VMState  // virtual machine states

// rules
var k int               // some block number

∀ k:
  assert(vmStates[k + 1] == NextVMState(&vmStates[k], blocks[k].Txs))
```

Once the block is processed by the WebAssembly VM, it has to be stored in Swarm for the future batch validation. Blocks are stored in two separate pieces in Swarm: the block manifest and the transactions list. The manifest contains the Swarm hash of the transactions list, which makes it possible to find transactions by having just the manifest.

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

// data
var blocks    []Block                 // Tendermint blockchain
var vmStates  []VMState               // virtual machine states
var manifests []Manifest              // manifests
var swarm     map[[]byte]interface{}  // Swarm storage: hash(x) –> x

// rules
var k int                             // some block number

∀ k:
  assert(manifests[k].Header == blocks[k].Header)
  assert(manifests[k].LastCommit == blocks[k].LastCommit)
  assert(manifests[k].TxsSwarmHash == SwarmHash(blocks[k].Txs))
  assert(manifests[k].VMStateHash == MerkleRoot(vmStates[k].Chunks))
  assert(manifests[k].LastManifestSwarmHash == SwarmHash(manifests[k - 1]))

  assert(swarm[SwarmHash(manifests[k])] == manifest[k])
  assert(swarm[SwarmHash(blocks[k].Txs)] == blocks[k].Txs)
```

Now, once the block manifest is formed and the virtual machine has advanced to the new state, it becomes possible to compute the new application state hash, which will be used in the next block.

```go
∀ k:
  assert(blocks[k + 1].Header.AppHash == Hash(manifests[k]))
```

## Client

### Query results

Once the cluster has reached consensus on the block, advanced the virtual machine state, reached consensus on the next couple of blocks and saved related block manifests and transactions into Swarm, the client can query results of the function invocation through the ABCI query API. 

Let's assume that transaction sent by the client was included into the block `k`. In this case the client has to wait until the block `k + 2` is formed and the corresponding block manifest is uploaded to Swarm. Once this is done, results returned to the client will look the following.

```go
type QueryResults struct {
  Chunks           map[int]VMChunk  // selected virtual machine state chunks
  ChunksProofs     []MerkleProof    // Merkle proofs: chunks belong to the virtual machine state
  Manifests        [3]Manifest      // block manifests
  ManifestReceipts [3]SwarmReceipt  // Swarm receipts for block manifests
  TxsReceipt       SwarmReceipt     // Swarm receipt for block transactions
}

// data
var swarmContract   SwarmContract    // Swarm Ethereum smart contract

var blocks          []Block          // Tendermint blockchain
var vmStates        []VMState        // virtual machine states
var manifests       []Manifest       // manifests for blocks stored in Swarm

// rules
var k       int                      // some block number
var t       int                      // some virtual machine state chunk number
var p       int                      // some manifest index

var results QueryResults             // results returned for the block `k`

∀ k:
  ∀ t ∈ range results.Chunks: 
    assert(results.Chunks[t] == vmStates[k + 1].Chunks[t])
    assert(
      results.ChunksProofs[t] == 
      CreateMerkleProof(
        t,                      // index
        results.Chunks[t],      // selected chunk
        vmStates[k + 1].Chunks  // all chunks
      )
    )
    
  ∀ p ∈ [0, 3):
    assert(results.Manifests[p] == manifests[k + p])
    assert(results.ManifestReceipts[p] == SwarmUpload(results.Manifest[p]))
    
  assert(results.TxsReceipt == SwarmUpload(blocks[k].Txs))
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