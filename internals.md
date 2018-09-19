This is an outline of the current architectural decisions and internal details.  
With the time, the details will get expanded and open questions will get resolved :)


- [Networking](#networking)
- [Deep storage](#deep-storage)
- [Computation machine](#computation-machine)
  - [Introduction](#introduction)
  - [API](#api)
  - [Verification game](#verification-game)
  - [VM choice](#vm-choice)
  - [VM details](#vm-details)
  - [Attacks](#attacks)
- [Real-time computation](#real-time-computation)
  - [Introduction](#introduction)
    - [Cluster](#cluster)
    - [Client](#client)
  - [Computational framework](#computational-framework)
    - [API](#api)
    - [Consensus engine](#consensus-engine)
    - [Data processing engine](#data-processing-engine)
    - [Root chain](#root-chain)
    - [Operations](#operations)
    - [Client-side verification](#client-side-verification)
  - [Attacks](#attacks)
- [Batch validation](#batch-validation)
  - [Introduction](#introduction)
  - [Process](#process)
  - [Extra perks](#extra-perks)


# Networking

- **Kademlia p2p**
  - key-based routing & DHT table
  - XOR distance metric for nodes keys and stored data keys
  - proof-of-work crypto puzzle and security deposits required (avoiding Sybil/Eclipse attacks)
  - new node traverses the network recursively to be added
  - cached data is stored on the nodes in the neighborhood of the key
  - cached data is being republished to keep being stored in the right neighborhood
- **ecosystem integration**
  - certain technologies like Tendermint, Swarm or light Ethereum client might use own routing
  - Kademlia connects different parts of the stack: real-time nodes, clients and batch validators



# Deep storage
- **Swarm introduction**
  - hash-addressable storage: `hash(value) => value` storage
  - mutable resource update protocol: `key => value` storage
  - content insurance
  - receipts for stored data
- **receipts storage dilemma**
  - certain operations (MRU) might generate lots of receipts which have to be stored somewhere
  - can't store receipts in Swarm itself: need to claim insurance if Swarm is compromised
  - can't update the root chain with receipts too often: too expensive
  - can't trust entities such as real-time cluster with receipts storage
  - technically can have much cheaper side chains but sounds like another moving part
- **receipts storage in Kademlia DHT**
  - **_updater_** stores receipts signed with it's private key in Kademlia DHT 
  - receipts are stored on `~m` nodes closest to the `hash(key)` by Kademlia distance
  - **_verifier_** immediately checks `α * m` nodes to confirm they have actually stored receipts
  - probability of losing receipts is low enough: `~0.2 ^ m` assuming <20% compromised nodes
  - **_reader_** reads receipts from the nodes using Kademlia lookup
- **versioned resource update**
  - linked list storage: `v0 <- v1 <- v2 <- v3 <=== key`
    - value `v_k` contains a link and a receipt for the previous value `v_k-1`
    - key points to the latest value  
  - how this structure is stored in Swarm
  - **_updater_** stores receipts on `~m` nodes closest to the `hash(key)`
  - **_verifier_** immediately checks `α * m` nodes to confirm they have actually stored receipts
  - only the latest version of receipt is stored  
  - nodes periodically (infrequent enough) checkpoint receipts to the root-chain
  - the presence of receipts in Kademlia prevents **_updater_** from creating a fork
- **Kademlia operations**
  - **[???]** receipts are republished within Kademlia neighboorhood
  - **[???]** certain nodes receive a reward from smart contract for checkpointing
- **long-lived clients**
  - some clients might live long enough to get back and check correctness of the block
  - **[???]** for short-lived clients, fishermen might do the same
- **motivation and attacks**
  - **[???]** what economically motivates nodes to store receipts?
  - **[???]** how does the threat model look like?



# Computation machine

## Introduction
- **functions**
  - function: `f(x1, x2, ..., xN) -> y`
  - functions might have side effects and modify state
  - meta state: gas or instructions counter
- **computational entities**
  - real-time nodes (clusters): the entire data is local and immediately available
  - batch validators: need to download the data first
  - root chain (judge): able to operate only over the limited portion of the data 
- **disputes**
  - between real-time nodes
  - between real-time nodes and batch validators
  - resolved by the root chain (judge)
- **verification game**
  - need to provide the root chain only the essence of the dispute
  - the first instruction where the computation has diverged
  - instruction + used memory blocks
  - binary search
- **virtual machine**
  - simple enough to be implemented with the selected root chain smart contracts
  - fast enough when normally running on the real-time nodes / batch validators
  - WebAssembly _specification_ has a number of implementations
  - WebAssembly state: memory + stack
  - hierarchical hash computation for memory (Merkle tree)


## API
- **wasm contracts**
  - different languages support
  - WebAssembly code as a source of truth
  - **[???]** application binary interface: string input, string output + parsing in wasm
  - **[???]** state persistence (transient + persistent vars VS all vars persistent)
  - **[???]** interaction with the external world (ethereum? swarm?)
- **price model**
  - **[???]** price for having wasm memory allocated during the unit of time
  - **[???]** price for instructions execution
  - **[???]** price for memory chunks dirtying


## Verification game
- **diverged computations**
  - difference in VM states (memory/stack)
  - difference in gas/instructions counters
  - difference in returned results
  - any difference results in a dispute
- **disparity search**
  - binary search for a diverged wasm instruction
  - comparison of _total state_ (VM state + meta state + results) hashes  
  - §§§
    - different number of steps: `0 1 2 3 4` vs `0 1 2`
    - check step `1`
    - if state after `1` is different – verification game for `0 1`
    - if state after `1` is the same – check `2` with judge (`jne` or `jmp` incorrect behavior)
- **special case: out of gas dispute**
  - two executors: A and B
  - A incorrectly computed gas used (too little gas)
  - B can be paid only for this gas
  - B notices it ran out of gas and initiates a dispute for the part of the computation
  - this requires A to write down the gas used and sign it
- **dispute submission**
  - single instruction touches very little memory (reads/writes) 
  - total state is split into small chunks
  - total state hash is calculated 
  - only chunks used by an instruction are passed to the root chain
  - §§§
    - instruction pointer is also submitted
    - Merkle proofs for chunks are submitted
    - judge already has entire wasm code
    - **[???]** wasm code is split into chunks
- **optimizations**
  - §§§
    - **[???]** multiple passes to compute intermediate state, how to reduce complexity?
    - **[???]** multiple ways to compute Merkle root
      - full: on each bisect step – `O(log2(ins) * full_merkle_hash(mem))`
      - incremental: on each instruction – `O(ins * incremental_merkle_hash(mem))`
- **offline verification game**
  - **[???]** multiple instructions execution on the root chain


## VM choice
- **performance requirements**
  - normal mode: performance close to commonly used platforms (Java/Go/C++)
  - dispute mode: okay to be an order of magnitude slower
- **Asmble choice**
  - translates wasm to JVM bytecode, which can be executed by JVM (JIT included)
  - wasm modules become JVM classes, accepting a ByteBuffer in a constructor as wasm memory
  - wasm VM is quite close to JVM
    - **[???]** JVM stack might be used implicitly
    - **[???]** other similarities
- **Solidity choice**
  - root chain wasm interpreter / stepper


## VM details
- **errors handling**
  - **[???]** software transactional memory (Merkle tree memory copy-on-write)
- **Asmble customizations**
  - gas & instructions counting
  - determinism (esp. floating point)  
- **normal mode**
  - memory: Merkle root computation per **n** functions
  - stack: JVM stack
  - **[???]** gas & instructions counting: base block accounting level
- **dispute mode**
  - **[???]** memory: Merkle root computation per **n** functions
  - **[???]** stack: shadow stack
  - gas & instructions counting: instruction accounting level


## Attacks
- **[???]** wasm code VM escape
  - guarded by wasm validator
  - guarded by JVM sandboxing
  - guarded by Docker container
- **[???]** actual instructions cost might be way different from the gas model
- **[???]** _cache-unfriendly_ algorithms might cause expensive Merkle root recomputes
- **[???]** non-deterministic wasm code might cause chaos during execution
- **[???]** both real-time and batch nodes are motivated to overstate gas used



# Real-time computation

## Introduction

### Cluster
- **computational machine**
  - few (4-25) identical stateful nodes bridged by consensus
  - nodes are running daemonized wasm code
  - single cluster runs only one wasm package
  - wasm code is accessible from the outside via designated entry points
  - data locality: few GBs of data (state) + wasm code
- **consensus & replication**
  - full replication across the cluster
  - cluster reaches consensus on how the state should advance
  - every node that approved state advance bears responsibility
  - if any node disagrees it can raise a dispute right away
- **eventual security**
  - permissioned (root chain smart contracts setup permissions)
  - assumption: root chain never loses data and is trustworthy
  - security deposits with the smart contracts
  - stores the history of operations in a decentralized storage    
  - eventual security by the batch validation of the history
  - trading `security` for `real-time` & `efficiency` (and compensation)

### Client
- **extremely light**
  - light client: no data or code stored
  - **[???]** might be launched from the browser w/o special setup
  - authorized (with root chain smart contract) clients only OR general public
  - interacts with cluster nodes and root chain
- **security checks**
  - nodes participating in consensus and their security deposits
  - consensus agreement on returned results
  - batch validation lag
  - expected guarantees


## Computational framework

### API
- **function calls**
  - client invokes wasm code entry point functions
  - client sends input arguments, receives output results
  - functions might be effectful (modifying state) or not
- **happens-before semantic**
  - client has a number of sessions ("threads")
  - each session runs a code interacting with the cluster
  - happens-before guarantees for statements within a single session
  - **[???]** no happens-before guarantees (yet?) across different sessions or clients

### Consensus engine
- **Tendermint BFT consensus**
  - transaction: function name + argument values
  - transactions are processed by Tendermint BFT consensus engine
  - brief Tendermint introduction
  - Tendermint decides which transactions are included in the next block
  - application decides how the state should be changed by transactions
- **missing Tendermint functionality**
  - **[???]** Tendermint doesn't support happens-before relationships or ordering guarantees
  - Tendermint has separate write (_transactions_) and read (_queries_) requests support
  - Tendermint doesn't check transactions validity or client authorization

### Data processing engine
- **external gateway**
  - responsible for parsing transactions and auth-ing clients
  - §§§
    - **[???]** deduplicates transactions (e.g same one sent to different nodes) and protects agains replays
    - **[???]** transactions parsing happens inside wasm VM to have an ability to dispute
    - **[???]** clients auth happens inside wasm VM to have an ability to dispute
- **function results storage**
  - two aspects of a function: change state & return result
  - change state through Tendermint transactions
  - can't return results via transactions, instead stored as a part of state change (Tendermint app hash)
  - §§§
    - **[???]** circular buffer is used for results to avoid overflow
    - **[???]** results are placed inside wasm VM to participate in incorrect computation disputes
    - **[???]** results too big are forbidden
  - Tendermint Query interface can retrieve stored results
- **ordering**
  - transactions ordering pool: by client & by session
  - **[???]** transactions counters, client & sessions identifiers
  - **[???]** limits on the number and size of stored transactions to prevent memory exhaustion
  - **[???]** transactions are placed inside wasm VM to participate in incorrect computation disputes
- **transactions history storage**
  - **[???]** transactions stored in Swarm: a decentralized deep storage (last hash? MRU?)
  - brief Swarm introduction
  - state snapshot (for batch validation & fast recovery) – details later in **§ Batch validation**
  - **[???]** receipts for saved transactions which are stored in the root chain
  - **[???]** receipts for saved transactions also participate in the app hash (block) computation
  - **[???]** receipts might be used by batch validators to retrieve the data
  - **[???]** receipts might be used by the client to make sure that batch validation will happen
  - latest snapshot + transactions history might be used for recovery
  - **[???]** dealing with potential data losses
- **disputes**
  - dispute: escalating to an external trusted judge on the root chain
  - stop-the-world disputes mechanism
  - synchronous disputes (within consensus) and their guarantees
  - asynchronous disputes (<1/3 nodes that were late for consensus) and their guarantees
  - security deposits as collaterals
- **payments**
  - **[???]** gas model
  - **[???]** interactive (unlimited) gas model
  - **[???]** check books, payment channels, payments flow
  - **[???]** batch validation and checks cashing time limits

### Root chain
- **cluster formation**
  - **[???]** wasm code uploading
  - **[???]** public keys and signatures
  - **[???]** security deposits and initial balances
  - **[???]** nodes location (Kademlia? ip addresses?)
- **cluster membership changes**
  - **[???]** client notification
  - **[???]** Tendermint cluster update
  - **[???]** node initialization / recovery process using **§ transactions history storage**
- **light client aspects**
  - **[???]** light client security guarantees
  - **[???]** forked tail and orphaned blocks
  - **[???]** single malicious node connection

### Operations
- **Tendermint compaction**
  - **[???]** cluster recreation from scratch
  - **[???]** marking k-th block as a head in root chain
- **node persistence**
  - **[???]** virtual machine memory snapshots
  - **[???]** Tendermint transactions log replay

### Client-side verification
- **security checks**
  - **[???]** nodes participating in consensus (using root chain light client)
  - security deposits (guarantees reasoning)
  - results returned by the cluster node are established by consensus
  - **[???]** the history of operations is actually stored for batch validation
  - **[???]** batch validation lag (guarantees reasoning)
  - **[???]** transactions and results ordering


## Attacks
- **[???]** DDOS: too many transactions to the single node (possibly incorrect ones)
- **[???]** DDOS: too many transactions to the cluster (passing initial transaction checking)
- **[???]** DDOS: a client might take nodes memory with too many out-of-order transactions
- **[???]** DDOS: a client might take nodes memory with too many sessions
- **[???]** DDOS: an attacker might create too many clients (general public case)
- **[???]** DDOS: too many Query requests
- **[???]** malicious dispute: bad node might stop real-time processing with constant disputes
- **[???]** bad client: client claims it has submitted a transaction while in fact it has not
- **[???]** sabotage: a node might silently drop a transaction
- **[???]** sabotage: a node might claim a transaction is invalid with VM-outside verification
  - client authentication/authorization
  - bad request: parse errors, invalid or incompatible arguments
- **[???]** hollow work: a node might behave like general clients and send lots of txs to get paid
- **[???]** fraud: a node might return incorrect results with VM-outside verification
- **[???]** fake: a node (or general attacker) might send a bogus transaction
- **[???]** replay: a node (or general attacker) might repeat a transaction
- **[???]** data loss: Swarm nodes might collude with Fluence nodes to drop transactions history
- **[???]** stale responses: the cluster might return stale responses for light client reads
- **[???]** dependency: potential attacks affecting Ethereum clients
- **[???]** dependency: potential attacks affecting Swarm clients
- **[???]** existing: attacks similar to attacks on TrueBit economy
- **[???]** Sybil: attacker creating lots of bogus nodes
- **[???]** VM escape: wasm code might exploit VM vulnerabilities to damage host machine
- **[???]** cluster takeover: >2/3 malicious nodes might even stop talking to the honest ones
- **[???]** cluster takeover: >1/3 malicious nodes might hurt cluster operations



# Batch validation

## Introduction
- **motivation**
  - real-time demands data locality
  - need to balance between security and efficiency
  - too few real-time nodes repeating the computation: high probability of cluster takeover
  - too little randomness when verifying computations: easy collusion
  - independent verifying nodes are needed
- **process overview**
  - batch validator is chosen
  - batch validator retrieves the previous snapshot from Swarm
  - batch validator replays transactions from the log
  - state after each transaction is checked and disputes submitted in the case of discrepancy  
  - new snapshot is uploaded to Swarm and published about into root chain
  - if no disputes, the validator places a security deposit as an additional guarantee


## Process
- **work discovery**
  - batch validators are running as daemons constantly monitoring available work
  - to find work validators listen for checkpoint root chain contract events (**§Deep storage integration**)
  - on launch batch validators register in the network and submit a security deposit  
  - **[???]** only a random subset of validators is allowed to validate a specific cluster to avoid cartels
  - validator goes backwards through checkpoints, noting number of validations on each checkpoint
  - every validated checkpoint must have corresponding snapshot and a number of accomplished (done/made) validations
- **work selection**
  - checkpoints that don't have preceding snapshots are skipped to the previous checkpoints
  - validator receives fixed compensation for downloading `snapshot + blocks` from storage
  - validator receives `(gas * price) / 2^(n + 1)` payout, `n` – number of existing validations
  - validator checks statistics of other validators to predict the competition
  - if there is too much competition it withdraws from entering
  - validator publishes ([???] Kademlia/root chain) that it has commited to validate a specific work
- **disputes**
  - validator initiates a dispute for a specific block in case of diverged block hashes
  - cluster downloads the closest previous snapshot and rolls `k` blocks until finding diverged one
  - verification game between the cluster and the validator commences
  - if the validator loses, cluster collects a hefty fee for lots of unnecessary work performed
  - all of the cluster nodes that signed incorrect block are punished in case of lost dispute
  - all previous validators signed incorrect snapshot are punished as well
- **finalization**
  - once validator has completed the processing, it saves a snapshot if it doesn't exist
  - otherwise it checks it's snapshot hash against saved one and increments validation counter
  - validator uploads receipts of the snapshot to the root chain
  - validator notifies the root chain that it has verified the period of time  
  - validator's security deposit is locked for a period of time
  - the client is able to check the current validation state
- **[???]** **forced errors, proof of independent execution**
- **payment distribution**
  - real-time cluster estimates gas used which also reserves gas for batch validation
  - real-time cluster writes down the gas used in the blocks  


## Extra perks
- **real-time cluster operations**
  - real-time nodes replacement
  - no state download burden on the real-time cluster
  - new coming node downloads the latest snapshot, then applies transactions from the log
- **historical analysis**
  - ability to efficiently restore the state at any _recent_ point in time
  - download the closest snapshot and apply transactions up to the desired time
