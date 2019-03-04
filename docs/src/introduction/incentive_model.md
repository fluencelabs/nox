# Incentive model

Fluence uses a concept similar to Ethereum gas to track computational efforts. With few exceptions, every WebAssembly instruction has a predefined associated cost, which is named fuel to avoid confusion with Ethereum gas. The fuel required to perform a computation is roughly proportional to the total sum of fuel amounts assigned to instructions in the computation execution trace.

When it comes to storage usage accounting, Fluence rules differ significantly from Ethereum. Ethereum instructions for interacting with persistent storage require the client to pay a one-time fee – which is significant – to compensate for a certain amount of future expenses that will be incurred in storing the information. For example, `SSTORE` (the instruction that is used to save a word to the persistent storage) costs 20,000 gas, which is a few orders of magnitude more than the cost of basic instructions such as `POP`, `ADD`, or `MUL`, which require 2–5 gas.

While this approach has been working fairly well for Ethereum, we think that adapting it to the Fluence network – the aim of which is achieving cost-efficiency comparable to traditional clouds – is problematic. In conventional backend software, it is common to update an on-disk or in-memory state without worrying that the performed update costs considerably more than other operations. In order to easily port existing software to Fluence, we need to provide developers a method that does not require them to fundamentally modify the code being ported.

Another reason to reconsider storage accounting is that execution of WebAssembly instructions and provision of data storage are quite different. If we say that network nodes are compensated for the performed work, then the total difficulty of processed instructions indeed defines an amount of the work performed. However, allocating a megabyte of storage is not _work_ – it is _power_. Only after a node has kept a megabyte of data in storage for a certain time can we estimate how much work it has performed: _work_ = _power_ · _time_.

When a client pays a one-time upfront fee to upload their data, there is no way for the network node responsible for its storage to know how long the data will be stored. No matter how large the upfront fee is, it is possible that expenses required to store the data will exceed this fee, leaving the financial burden on the node. This means that a different storage accounting approach must be developed for the Fluence network, which we propose and discuss below.

**Rewards accounting.** To counteract the aforementioned issues, various storage rent fees were proposed for Ethereum, including requiring clients to pay a fee to renew their storage every time they issue a transaction. However, to bring the developer experience as close as possible to traditional backend software, in the Fluence network, the developer is the only party finally responsible for compensating network nodes.

Fluence nodes are compensated for the computational difficulty of executed WebAssembly instructions and for the storage space allocated for a specific period of time. Because different hardware might need different time to execute the same program, computational difficulty is used as a sub- stitution for time. In other words, once a block of client transactions is processed and the fuel _φ_ required to process it is counted, this fuel is transformed into the standard time _t_<sub>std</sub> by multiplying it by the network-wide scaling constant _c_<sub>time/fuel</sub>:

<center>
_t_<sub>std</sub> = _c_<sub>time/fuel</sub> · _φ_
<br><br>
</center>

To estimate the total node reward _υ_, two more scaling constants are introduced: _c_<sub>υ/fuel</sub> converts spent units of fuel into the network currency; _c_<sub>υ/spacetime</sub> does the same with the unit of storage space allocated for the unit of time. does the same with storage space per time. Assuming that the size of the allocated storage space is denoted by _ω_, the total node reward is computed as:

<center>
_υ_ = _c_<sub>υ/fuel</sub> · _φ_ + _c_<sub>υ/spacetime</sub> · _ω_ · _t_<sub>std</sub>
<br><br>
</center>

It should be noted that contrary to the system used by Ethereum where a client is able to choose a different gas price for every transaction, the scaling constants _c_<sub>time/fuel</sub>, _c_<sub>υ/fuel</sub>, and _c_<sub>υ/spacetime</sub> are fixed for the entire Fluence network. One reason for this design is that batch validators are selected randomly and are not able to choose the computations they are going to verify.

By allowing clients or developers to choose their compensation level, batch validators might be forced to perform complex computations for an unreasonably low reward. To prevent this, scaling constants are periodically updated, similar to how mining difficulty changes in Ethereum. If there is not enough supply, the network-wide compensation level increases; conversely, if there is not enough demand, the compensation level drops.

**Dummy transactions.** Because time is counted only for performed computations, simply storing the state without processing transaction blocks does not ensure any compensation to real-time workers. Therefore, in cases where incoming client transactions are rare, it is possible that the block creation rate will be low and thus low-demand backends will spend lots of time storing the state between the blocks. This time will never be compensated; workers running such low-demand backends might spend far more resources to store the state than their total compensation will be.

To offset this, real-time workers are allowed to send dummy transactions to themselves; the fuel required to process such dummy transactions is accounted for in the same way as the fuel required to process client transactions. This way, even if the client transaction volume is low, real-time workers will be compensated proportionally to the (real-world) time they have been running a certain backend deployment.

Batch validators, however, are not affected by this issue because they do not have to wait for incoming transactions and new blocks. A batch validator replays a fragment of transaction history at the maximum rate it is able to perform; once it completes the processing of the fragment, it moves to the next fragment. Additionally, for the same amount of work (which is defined by used fuel), real-time workers and batch validators are compensated evenly, which makes both options equally attractive to miners. Therefore, no special mechanism to recompense batch validators exists in the Fluence network.

Because different hardware can process transactions at different rates, it might happen that a very fast real-time cluster will be able to produce and process dummy transactions so fast that the compensation from the developer will become unexpectedly high. To mitigate this, in addition to being able to set the size of the storage space _ω_, developers have the ability to set the maximum fuel amount _φ_<sub>max</sub> that real-time workers are allowed to spend per unit of time.

This allows a developer to budget how much will be spent on computations performed by the Fluence network in the next day, week, or month. Additionally, it lets real-time workers plan how much capacity they should allocate for transaction processing performed by the backend deployed by that developer.

We should note that it is possible for ill-disposed real-time workers to process only self-created dummy transactions – and none sent by clients. While we do not discuss a detailed mechanism to combat such behavior in this paper, we should note that a developer monitoring the use of the deployed backend will be able to notice a drop in the ratio between client and dummy transactions. In this case, the developer can either replace misbehaving real-time workers with other network nodes, or reduce the amount of fuel _φ_<sub>max</sub> that the real-time cluster is allowed to spend per unit of time.

**Client billing.** As we have previously mentioned, developers are exclusively responsible for paying out rewards to the Fluence network nodes. Because miners’ compensation is proportional to used fuel and allocated storage space, developers are directly incentivized to write efficient backends.

Generally, clients are not responsible for making any payments to network nodes. Nevertheless, different external monetization schemes are possible for reimbursing developers. The Fluence network does not prescribe an exact monetization scheme; however, it might provide some of the most common schemes through extension packages.

For example, one developer might allow only those clients from a whitelist to interact with the deployed backend, charging a flat rate to add a client to that whitelist; another developer might charge clients a fixed fee per each submitted transaction. It might also be possible for clients to pay no explicit fee while the developer uses their personal funds to cover the miners’ expenses.