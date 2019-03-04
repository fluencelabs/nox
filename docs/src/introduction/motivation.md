# Motivation

To understand the reason behind having two noticeably different layers, we need to recall their properties. Real-time workers are stateful, which considerably improves response latencies because they do not have to download the required state data to perform computations.
As an example, assume that we are building a decentralized SQL database that should support an indexed access to data. Complex queries such as the one listed below often require traversal of multiple indices, which are often implemented as B-trees.

```sql
SELECT
    DATE(ts) AS date,
    AVG(gas_price * gas_used) AS tx_cost
FROM transactions tx WHERE
    tx.to IN (SELECT address
              FROM contracts
              WHERE name = ’CryptoDolphins’) 
GROUP BY DATE(ts)
```
_Example query to blockchain data._


To traverse a B-tree, we need to sequentially fetch its nodes that satisfy the query conditions. It is not possible to retrieve the required B-tree nodes all at once because the next node to fetch can be determined only by matching the parent B-tree node against the query. If an index is stored externally in a decentralized storage such as IPFS or Swarm, this means that multiple network roundtrips must be performed between the machine performing the computations and the data storage, which significantly increases latency.

Other algorithms, especially those requiring irregular access to data, could benefit from storing their data locally as well. However, data locality significantly increases the economic barrier to joining a real-time cluster because a worker willing to participate in the cluster has to download the current state first. Consequently, this motivates workers to remain in the cluster and thus cluster compositions do not change much over time.

This means that malicious real-time workers in the cluster might form a cartel to produce incorrect results. Without batch validation, every node in the real-time cluster knows it will be verified only by its peers, which are known in advance because clusters are tightly connected. Consequently, malicious nodes can, for example, exploit the following strategy: use a special handshake to recognize other malicious nodes in the cluster and start producing incorrect results if they account for at least \\(\frac{2}{3}\\) of the total number of nodes (so they can reach BFT consensus without talking to the rest of the cluster); otherwise, work honestly. This strategy is virtually impossible to catch without external verification.

Furthermore, because real-time clusters are supposed to be small enough to be cost-efficient, the probability that malicious nodes will take over a cluster is significant. For example, for a network where 10% of all nodes are malicious, a real-time cluster that consists of 7 workers independently sampled from the network has approximately a 1.8 · 10<sup>−4</sup> chance to have at least \\(\frac{2}{3}\\) malicious nodes.

To counteract this, the batch validation layer provides external verification. Nodes performing batch validation are chosen randomly, which means real-time nodes do not know beforehand which validator will be verifying them and thus cannot collude with the validator in advance.

Batch validation also decreases the probability that an intentional mistake made by a real-time cluster will never get noticed. Assume that in the same network where 10% of all nodes are malicious, we spun a real-time cluster of 4 workers and allocated a budget for 3 batch validations.

In this setup, a mistake can go unnoticed only if malicious actors comprise at least \\(\frac{2}{3}\\) of the realtime workers and all of the batch validators that verified the transaction history. The chance of this happening is ≈ 3.7 · 10<sup>−6</sup>, which is two orders of magnitude less than in the case where the entire budget was spent on the real-time cluster only.

We also expect that in the presence of batch validators, the fraction of malicious nodes in the network will drop significantly below 10%, because every time a malicious action is caught, the node that performed it loses its deposit and thus leaves the network.