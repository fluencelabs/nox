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
  - [Query response](#query-response)
- [Client](#client)  
  - [Query response verification](#query-response-verification)
- [Batch validation](#batch-validation)

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
  assertTrue(exampleContract.Collaterals[nodeId] >= minCollateral)
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
  assertEq(swarm[SwarmHash(content)], content)
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

// downloads the content from the Swarm network using the supplied receipt
func SwarmDownload(receipt SwarmReceipt) []byte {}

// data
var content []byte // some content

// rules
∀ content:
  var receipt = SwarmUpload(content)

  assertEq(receipt.ContentHash, SwarmHash(content))
  assertTrue(SwarmVerify(receipt.Insurance, receipt.ContentHash))
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

  assertEq(meta.Key, key)
  assertEq(meta.Version, version)
  assertEq(meta.Permission.PublicKey, swarmOwners.Owner(meta.Key))
  assertTrue(SwarmVerify(meta.Permission, SwarmHash(pack(meta.Key, meta.Version, meta.Content))))
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
  assertEq(receipt.ContentHash, SwarmHash(pack(meta)))
```

### Kademlia sidechain

In fact, Kademlia is not quite an external system but is an essential foundation of the Fluence network. However, for our protocol we use it as a some kind of a sidechain which periodically checkpoints selected blocks to the root chain – Ethereum. Blocks stored in Kademlia sidechain are really tiny compared to the overall data volumes flowing through the network – they contain only Swarm receipts and producers signatures.

```go
type SideBlock struct {
  Height        int64         // side block height
  PrevBlockHash Digest        // hash of the previous side block
  Receipt       SwarmReceipt  // Swarm receipt for the content associated with the side block
  Signatures    []Seal        // signatures of the side block producers
}

type SideContract struct {
  CheckpointInterval int          // how often blocks should be checkpointed
  Checkpoints        []SideBlock  // block checkpoints
}
```

It is expected that every Kademlia sidechain node stores the tail of the chain starting from the last block checkpointed into the contract. Every node verifies there are no forks or incorrect references in the chain tail – otherwise, a dispute is submitted to the contract and offending block producers lose their deposits.

<p align="center">
  <img src="images/sidechain.png" alt="Sidechain" width="794px"/>
</p>

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
func (contract *SideContract) DisputeSideCheckpoint(index int, blocks []SideBlock) {
  var prevCheckpoint = contract.Checkpoints[index - 1]
  var checkpoint = contract.Checkpoints[index]

  // checking that the chain is linked correctly
  for i, block := range blocks {
    var prevBlock SideBlock
    if i == 0 { prevBlock = prevCheckpoint } else { prevBlock = blocks[i - 1]}
    if block.PrevBlockHash != Hash(pack(prevBlock)) || (block.Height != prevBlock.Height + 1) {
      // incorrect chain segment, nothing to do here
      return
    }
  }

  if Hash(pack(blocks[len(blocks) - 1])) != Hash(pack(checkpoint)) {
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
func (node *SideNode) UploadBlock(block SideBlock) Seal {}
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

type Transactions = []Transaction
```

## Real-time processing

### Transaction validation

Once the client has constructed a transaction, it is submitted to one of the real-time nodes which checks the received transaction:

```go
// verifies that transaction was originated by the client with enough funds deposited
func VerifyTransaction(flnContract *FlnContract, tx *Transaction, minCollateral int64){
  // checking that the client actually exists in the contract
  collateral, ok := flnContract.ClientCollaterals[tx.Seal.PublicKey]
  assertTrue(ok)

  // checking that the client has enough funds
  assertTrue(collateral >= minCollateral)

  // checking that the transaction is signed by this client
  assertTrue(Verify(tx.Seal, Hash(tx.Invoke)))
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
func TmHash(data []byte) Digest {}
func TmSign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal {}
func TmVerify(seal Seal, digest Digest) bool {}
func TmMerkleRoot(allChunks []Chunk) Digest {}
```

Tendermint periodically pulls few transactions from the mempool and forms a new block. Nodes participating in consensus sign produced blocks, however their signatures for a specific block are available only as a part of the next block.

```go
type Block struct {
  Header     Header         // block header
  LastCommit []Seal         // Tendermint nodes votes for the previous block
  Txs        Transactions   // transactions as sent by clients
}

type Header struct {
  LastBlockHash  Digest  // Merkle root of the previous block header fields
  LastCommitHash Digest  // Merkle root of the last commit votes
  TxsHash        Digest  // Merkle root of the block transactions
  AppHash        Digest  // application state hash after the previous block
}

// data
var blocks []Block // Tendermint blockchain

// rules
var k int64        // some block number
var i int          // some Tendermint node index

∀ k:
  assertEq(blocks[k].Header.LastBlockHash, TmMerkleRoot(packMulti(blocks[k - 1].Header)))
  assertEq(blocks[k].Header.LastCommitHash, TmMerkleRoot(packMulti(blocks[k].LastCommit)))
  assertEq(blocks[k].Header.TxsHash, TmMerkleRoot(packMulti(blocks[k].Txs)))
  
  ∀ i:
    assertTrue(TmVerify(blocks[k].LastCommit[i], blocks[k].Header.LastBlockHash))
```

Note we haven't specified here how the application state hash (`Header.AppHash`) is getting calculated – this will be described in the next section.

### Block processing

Once the block has passed through Tendermint consensus, it is delivered to the state machine. State machine passes block transactions to the WebAssembly VM causing the latter to change state. The virtual machine state is essentially a block of memory split into chunks which can be used to compute the virtual machine state hash. We can say that the virtual machine state `k + 1` is derived by applying transactions in the block `k` to the virtual machine state `k`.

```go
type VMState struct {
  Chunks []Chunk     // virtual machine memory chunks
}

// produces the new state by applying block transactions to the old VM state
func NextVMState(vmState VMState, txs Transactions) VMState {}
```

Once the block is processed by the WebAssembly VM, it has to be stored in Swarm for the future batch validation. Two separate pieces are actually stored in Swarm for each Tendermint block: the block manifest and the transactions list.

```go
type Manifest struct {
  Header              Header        // block header
  VMStateHash         Digest        // hash of the VM state derived by applying the block
  LastCommit          []Seal        // Tendermint nodes signatures for the previous block header
  TxsReceipt          SwarmReceipt  // Swarm hash of the block transactions
  LastManifestReceipt SwarmReceipt  // Swarm hash of the previous manifest
}
```

 Every manifest contains the Swarm hash of the transactions list, which makes it possible to find transactions by having just the manifest. Every manifest also contains the Swarm hash of the previous manifest which allows to retrieve the entire history by having just a single Swarm receipt for the latest manifest.

<p align="center">
  <img src="images/manifests_receipts.png" alt="Manifests Receipts" width="550px"/>
</p>

To create a manifest, the node splits the block into pieces, computes the hash of the new virtual machine state, uploads transactions to Swarm and adds links to the necessary Swarm content. Also, once the block manifest is crafted, it's hash is used as the new application state hash stored in the next block header. This way, because Tendermint nodes that have reached consensus sign the block header, they also confirm that the new manifest and the new virtual machine state were accepted.

```go
// returns the new virtual machine state, the manifest for the stored block and the next app hash
func ProcessBlock(block Block, prevVMState VMState, prevManifestReceipt SwarmReceipt,
) (VMState, Manifest, SwarmReceipt, Digest) {
  var vmState = NextVMState(prevVMState, block.Txs)
  var txsReceipt = SwarmUpload(pack(block.Txs))

  var manifest = Manifest{
    Header:              block.Header,
    VMStateHash:         MerkleRoot(vmState.Chunks),
    LastCommit:          block.LastCommit,
    TxsReceipt:          txsReceipt,
    LastManifestReceipt: prevManifestReceipt,
  }
  var receipt = SwarmUpload(pack(manifest))
  var nextAppHash = Hash(pack(manifest))

  return vmState, manifest, receipt, nextAppHash
}
```

### Query response

Once the cluster has reached consensus on the block, advanced the virtual machine state, reached consensus on the next couple of blocks and saved related block manifests and transactions into Swarm, the client can query results of the function invocation through the ABCI query API. 

Let's assume the transaction sent by the client was included into the block `k`. In this case the client has to wait until the block `k + 2` is formed and block manifests for the corresponding three blocks are uploaded to Swarm. Once this is done, the response returned to the client will look the following:

```go
type QueryResponse struct {
  Chunks    map[int]Chunk        // selected virtual machine state chunks
  Proofs    map[int]MerkleProof  // Merkle proofs: chunks belong to the virtual machine state
  Manifests [3]Manifest          // block manifests
}
```

Results obtained by invoking the function are stored as a part of the virtual machine state. This way, the node can return selected chunks of the virtual machine memory and supply Merkle proofs confirming these chunks indeed correspond to the correct virtual machine state Merkle root.

```go
// prepares the query response
func MakeQueryResponse(manifests [3]Manifest, vmState VMState, chunksIndices []int) QueryResponse {
  var chunks = make(map[int]Chunk)
  var proofs = make(map[int]MerkleProof)

  for _, index := range chunksIndices {
    var chunk = vmState.Chunks[index]

    chunks[index] = chunk
    proofs[index] = CreateMerkleProof(index, chunk, vmState.Chunks)
  }

  return QueryResponse{Chunks: chunks, Proofs: proofs, Manifests: manifests}
}
```

The reason why do we need multiple manifests in response is that nodes are required to prove that a consensus was reached on the manifest `k`. `AppHash` in the manifest `k + 1` header points to the manifest `k` content, and `LastBlockHash` in the manifest `k + 2` header points to the manifest `k + 1` header. Also, `LastCommit` field in the manifest `k + 2` provides nodes signatures for the `LastBlockHash`. A client can follow those links and verify the consistency of obtained results.

<p align="center">
  <img src="images/manifests_signatures.png" alt="Manifests Signatures" width="787px"/>
</p>

## Client

### Query response verification

The client verifies that returned response represents a correct block progress in a few steps. Below we will list those steps, but first we need to mention that they are not verifying that the transaction sent by the client was actually processed.

Instead, all the client does verify here is that the virtual machine state progress made by executing the block `k` was saved properly in Swarm for the future batch validation. In this case, if the state transition was performed incorrectly, real-time nodes deposits will be slashed.

However, an all-malicious cluster might never include the transaction sent by the client. In this case the new virtual machine state won't have the corresponding function return value. It might also happen that a malicious cluster will include an invalid transaction into the block. For example, that might be a transaction that was never originated by the correct client but still changing the state.

These aspects will be considered in another section, and for now we will focus on how the block progress is being verified.

#### Prerequisites

Below we assume that functions allowing to verify that Swarm receipts and Tendermint signatures were created by valid nodes already exist.

```go
func VerifySwarmReceipt(swarmContract SwarmContract, receipt SwarmReceipt) {
  var minCollateral int64 = 1000000

  // checking that the Swarm node has enough funds
  var swarmNodeId = receipt.Insurance.PublicKey
  assertTrue(swarmContract.Collaterals[swarmNodeId] >= minCollateral)

  // checking that the receipt is signed by this Swarm node
  assertTrue(SwarmVerify(receipt.Insurance, receipt.ContentHash))
}

func VerifyTendermintSignature(flnContract FlnContract, seal Seal, blockHash Digest) {
  var minCollateral int64 = 1000000

  // checking that the Tendermint node has enough funds
  var tmNodeId = seal.PublicKey
  assertTrue(flnContract.NodesCollaterals[tmNodeId] >= minCollateral)

  // checking that the receipt is signed by this Tendermint node
  assertTrue(TmVerify(seal, blockHash))
}
```

#### Verification of Swarm receipts

The client checks that every manifest is stored in Swarm properly. This means that each Swarm receipt – for the previous manifest or for the transactions block is signed by the Swarm node in good standing. It also means that manifests are connected together properly by Swarm receipts.

```go
func VerifyManifestsReceipts(swarmContract SwarmContract, response QueryResponse) {
  // checking that manifests and transactions receipts are properly signed by Swarm nodes
  for _, manifest := range response.Manifests {
    VerifySwarmReceipt(swarmContract, manifest.LastManifestReceipt)
    VerifySwarmReceipt(swarmContract, manifest.TxsReceipt)
  }

  // checking that each manifest points correctly to the previous manifest via the Swarm receipt
  for i := 0; i < 2; i++ {
    var manifest = response.Manifests[i + 1]
    var prevManifest = response.Manifests[i]

    assertEq(manifest.LastManifestReceipt.ContentHash, SwarmHash(pack(prevManifest)))
  }
}
```

#### Verification of consensus on the virtual machine state

The client checks that the chain linking Tendermint nodes signatures in the manifest `k + 2` with the virtual machine state hash in the manifest `k` is formed correctly. It also checks that the BFT consensus was reached by Tendermint nodes in good standing.

```go
func VerifyVMStateConsensus(flnContract FlnContract, response QueryResponse) {
  var manifests = response.Manifests

  // checking connection between the VM state in the manifest 0 and Tendermint signatures in the manifest 2
  assertEq(manifests[1].Header.AppHash, Hash(pack(manifests[0])))
  assertEq(manifests[2].Header.LastBlockHash, TmMerkleRoot(packMulti(manifests[1].Header)))

  // counting the number of unique Tendermint nodes public keys
  var lastCommitPublicKeys = make(map[PublicKey]bool)
  for _, seal := range manifests[2].LastCommit {
    lastCommitPublicKeys[seal.PublicKey] = true
  }

  // checking that BFT consensus was actually reached
  var signedNodes = float64(len(lastCommitPublicKeys))
  var requiredNodes = float64(2/3) * float64(len(flnContract.NodesCollaterals))
  assertTrue(signedNodes > requiredNodes)

  // checking each Tendermint node signature validity
  for _, seal := range manifests[2].LastCommit {
    VerifyTendermintSignature(flnContract, seal, manifests[2].Header.LastBlockHash)
  }
}
```

#### Verification of Merkle proofs for returned VM state chunks

Finally, the client checks that returned virtual machine state chunks indeed belong to the virtual machine state hash.

```go
func VerifyResponseChunks(results QueryResponse) {
  for k := range results.Chunks {
    assertTrue(VerifyMerkleProof(results.Chunks[k], results.Proofs[k], results.Manifests[0].VMStateHash))
  }
}
```

## Batch validation

Batch validators are able to locate blocks that should be checked using the checkpoints stored in Ethereum contract. The validator chooses one of the checkpoints and downloads the manifest complementary to it using the suitable Swarm receipt. Now, the validator can unwind the chain until the next checkpoint by following receipts stored in each manifest and also download corresponding transactions.

```go
func FetchSubchain(sideContract SideContract, index int) ([]Manifest, []Transactions) {
  var checkpoint = sideContract.Checkpoints[index]

  var count = sideContract.CheckpointInterval + 2
  var manifests = make([]Manifest, count)
  var txss = make([][]Transaction, count)

  var receipt = checkpoint.Receipt
  for i := count - 1; i >= 0; i-- {
    manifests[i] = ManifestUnpack(SwarmDownload(receipt))
    txss[i] = TransactionsUnpack(SwarmDownload(manifests[i].TxsReceipt))

    receipt = manifests[i].LastManifestReceipt
  }

  return manifests, txss
}
```

Here you can note that the number of manifests fetched exceeds the checkpoint interval by two. The reason is that, as we have mentioned in [§ Query response](#query-response) the block `k + 2` certifies the consensus on the block `k` virtual machine state. This means that checkpoint intervals and batch validation intervals are actually offset by two blocks.

<p align="center">
  <img src="images/batch_validation_interval.png" alt="Batch validation interval" width="771px"/>
</p>




