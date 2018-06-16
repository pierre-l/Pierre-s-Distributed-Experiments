The Blockchain Network Simulation
=================================

BNS is a simulation of a Proof-of-Work blockchain network built on top of a rudimental peer-to-peer network simulator. It is written in [Rust](https://www.rust-lang.org/en-US/) and relies on the [Tokio and futures libraries](https://tokio.rs/).

Status
------

This is an experimental project. Everything written here was not written for a production environment. Significant API changes are to be expected in the near future.

[![Build Status](https://travis-ci.org/pierre-l/blockchain_network_simulation.svg?branch=master)](https://travis-ci.org/pierre-l/blockchain_network_simulation)
[![License: MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg)](https://opensource.org/licenses/MIT)

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

License
-------

This project is licensed under the MIT license.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Tokio by you, shall be licensed as MIT, without any additional terms or conditions.