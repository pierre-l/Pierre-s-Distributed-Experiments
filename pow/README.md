Proof-of-Work Blockchain Network Simulation
===========================================

This simulation is part of the [Pierre's Distributed Experiments(PDE)](../). It was built on top of PDE's [Network Simulator](../network_simulator).

Features
--------

Simulates a Proof-of-Work blockchain network with thousand of nodes.
```
INFO 2018-06-21T21:45:32Z: Chain difficulty: 00007fffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
INFO 2018-06-21T21:45:33Z: [#0504] Mined a new block: 00007c1a4d17781f1765ae2b12528083e9381a5a09824ffe809079f5a3882275, height 1
INFO 2018-06-21T21:45:34Z: [#0090] Mined a new block: 0000738f72882544bbc5889ddd8d1292980d951441bd8ece83a32423d4223c47, height 2
INFO 2018-06-21T21:45:35Z: [#0740] Mined a new block: 000017751ed474dd6b6ee56485839b60a8c6ddb7ce508ceba7f85e688d6d4dda, height 3
INFO 2018-06-21T21:45:36Z: [#0473] Mined a new block: 000054fae7bd7677b26d841e2ad3109dd37669a1b2108ab0deb35263e5efee59, height 4
INFO 2018-06-21T21:45:36Z: [#1505] Mined a new block: 00003d1ce90d75ed55d6321a1b3f97e9eb9cb97716beeb1250c0e065e669b57c, height 5
INFO 2018-06-21T21:45:37Z: [#1301] Mined a new block: 0000371fd8fd747736f74e2cb1887122a3ba6e0dea5b1c2076a2e1da9defe5ef, height 6
```

Run the following command for a description of the parameters:
```
blockchain_network_simulation --help
```

How it works
---
Basic knowledge about proof-of-work blockchains and the Tokio library are recommended to deeply understand how this simulation works.

In this simulation, every blockchain node starts by mining blocks from the genesis block. It answers to every new connection with a status message containing the longest chain known by the node. When a new block is mined or received from a peer, this new chain is validated and compared to the longest known chain. If it is effectively longer then it is propagated to the miner and to the peers.

In the end, a consensus is reached quickly (every node has the same longest chain) and the chain is expanded further as time passes.

Limitations
-----------

This project inherits the benefits and limitations of PDE's [Network Simulator](../network_simulator).

An additional compromise is the delay enforced on mining iterations: a node will try to mine a new block every X milliseconds and not continuously.