# MULTISS - Arithmetics

## Introduction

This crate provides arithmetic operations for the MULTISS protocols, for secret storage across multiple remote QKD networks.
Standard and local-mode implementations are provided, the latter being used in case the number of nodes in the mother QKD network is equal to the number of daughter QKD networks.

It allows to test the protocol computations performance, for both secret distribution and recovery.

## Installation

### Rust installation

Install the Rust toolchain by following the instructions on the [official website](https://www.rust-lang.org/tools/install).

### Compilation

Build in release mode with the following command:

```bash
cargo build --release
```

## Testing

### Configuration

Bench configuration is done through JSON files, like [config-standard.json](./config-standard.json) and [config-local.json](./config-local.json).

Configuration fields are the following:
- `networks`: mode: `"standard"` or `"local"` mode of MULTISS
- `degree_p`: degree of the first polynomial *P* that is basically used to define the t~nets~ threshold.
- `networks`: array defining the QKD subnetworks (including the mother QKD network in standard mode), with the following fields:
  - `nodes`: number of nodes in the QKD subnetwork
  - `degree_q`: degree of the local polynomial, *Q~i~* for standard mode and *P~i~* for local mode. These polynomials define the *t~nodes~* threshold.
- `iterations`: number of iterations for the benchmark

:warning: **In standard mode, the topology of the mother QKD network is defined by the first element of `networks` list, while in local mode, the topology of the mother QKD network is not defined in the config, and is instead determined by the number of daughter QKD networks: the first network is first daughter.**

### Running the benchmarks

Run the benchmarks with the following command:

```bash
target/release/arithmetics <JSON config file>
```