The Blockchain Network Simulation
=================================

[![Build Status](https://travis-ci.org/pierre-l/blockchain_network_simulation.svg?branch=master)](https://travis-ci.org/pierre-l/blockchain_network_simulation)
[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)

BNS is a simulation of a Proof-of-Work blockchain network built on top of a rudimental peer-to-peer network simulator. It is written in [Rust](https://www.rust-lang.org/en-US/) and relies on the [Tokio and futures libraries](https://tokio.rs/).

Features
--------

Simulates a Proof-of-Work blockchain network with thousand of nodes.
```
INFO 2018-06-21T21:45:32Z: Chain difficulty: 00007fffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
INFO 2018-06-21T21:45:33Z: [#0504] Mined new block 00007c1a4d17781f1765ae2b12528083e9381a5a09824ffe809079f5a3882275, height 1
INFO 2018-06-21T21:45:34Z: [#0090] Mined new block 0000738f72882544bbc5889ddd8d1292980d951441bd8ece83a32423d4223c47, height 2
INFO 2018-06-21T21:45:35Z: [#0740] Mined new block 000017751ed474dd6b6ee56485839b60a8c6ddb7ce508ceba7f85e688d6d4dda, height 3
INFO 2018-06-21T21:45:36Z: [#0473] Mined new block 000054fae7bd7677b26d841e2ad3109dd37669a1b2108ab0deb35263e5efee59, height 4
INFO 2018-06-21T21:45:36Z: [#1505] Mined new block 00003d1ce90d75ed55d6321a1b3f97e9eb9cb97716beeb1250c0e065e669b57c, height 5
INFO 2018-06-21T21:45:37Z: [#1301] Mined new block 0000371fd8fd747736f74e2cb1887122a3ba6e0dea5b1c2076a2e1da9defe5ef, height 6
```

Run the following command for a description of the parameters:
```
blockchain_network_simulation --help
```

Status
------
This is an experimental project. Everything written here was not written for a production environment. Significant API changes are to be expected in the near future.

How it works
---
Basic knowledge about proof-of-work blockchains and the Tokio library are recommended to deeply understand how this simulation works.

The futures library provides [MPSC channels](https://docs.rs/futures/0.1/futures/sync/mpsc/fn.channel.html) with a similar interface to how Tokio would represent a standard TCP connection. BNS uses these channels to interconnect a pool of virtual nodes. Each of these nodes is always executed on the same thread by default, thus avoiding concurrent situations. Nodes are instructed to typically initiate a couple of connections to peers, avoiding network partitioning in standard cases.

In this simulation, every blockchain node starts by mining blocks from the genesis block. It answers to every new connection with a status message containing the longest chain known by the node. When a new block is mined or received from a peer, this new chain is validated and compared to the longest known chain. If it is effectively longer then it is propagated to the miner and to the peers.

In the end, a consensus is reached quickly (every node has the same longest chain) and the chain is expanded further as time passes.

Limitations
-----------
Since this is only a simulation, compromises were made in order to save resources and enable running semi-large scale networks. The use of MPSC channels instead of real TCP connections is the main one. This makes implementing serialization and discovery unnecessary but also enables sending pointers to immutable values instead of copying this values for every node, thus saving a lot of memory.

The main drawback of this is it does reproduce a much more idealistic situation than when using real TCP streams, and may therefore be more suitable for the study of distributed networks than for the practical design of one.

Another significant compromise is the delay enforced on mining iterations: a node will try to mine a new block every X milliseconds and not continuously.

License
-------
This project is licensed under the MIT license.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in BNS by you, shall be licensed as MIT, without any additional terms or conditions.