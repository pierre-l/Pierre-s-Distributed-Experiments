Network Simulator
=================

This simulation is part of the [Pierre's Distributed Experiments(PDE)](../). It relies on the [Tokio and futures libraries](https://tokio.rs/).

How it works
---
Basic knowledge about Rust and the Tokio library are recommended to deeply understand how this simulator works.

The futures library provides [MPSC channels](https://docs.rs/futures/0.1/futures/sync/mpsc/fn.channel.html) with a similar interface to how Tokio would represent a standard TCP connection. This simulator uses these channels to interconnect a pool of virtual nodes. Each of these nodes is always executed on the same thread by default, thus avoiding concurrent situations. Nodes are instructed to typically initiate a couple of connections to peers, avoiding network partitioning in standard cases.

Limitations
-----------
Since this is only a simulation, compromises were made in order to save resources and enable running semi-large scale networks. The use of MPSC channels instead of real TCP connections is the main one. This makes implementing serialization and discovery unnecessary but also enables sending pointers to immutable values instead of copying this values for every node, thus saving a lot of memory.

The main drawback of this is it does reproduce a much more idealistic situation than when using real TCP streams, and may therefore be more suitable for the study of distributed networks than for the practical design of one.