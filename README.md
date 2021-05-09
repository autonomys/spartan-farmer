<div align="center">
  <h1><code>spartan-farmer</code></h1>
  <strong>A proof-of-concept farmer for the <a href="https://subspace.network/">Subspace Network Blockchain</a></strong>
</div>

## Overview
**Notes:** The code is un-audited and not production ready, use it at your own risk.

Subspace is a proof-of-storage blockchain that resolves the farmer's dilemma, to learn more read our [whitepaper](https://drive.google.com/file/d/1v847u_XeVf0SBz7Y7LEMXi72QfqirstL/view). 

This a bare-bones farmer for the simplified Spartan proof-of-space variant of Subspace. A farmer is similar to a miner in a proof-of-work blockchain, but instead of wasting CPU cycles, it wastes disk space. Much of this code has been based on [subspace-core-rust](https://www.github.com/subspace/subspace-core-rust).

The farmer basically has two modes: plotting and farming.

### Plotting
1. A genesis piece is created from a short seed.
2. A new Schnorr key pair is generated, and the farmer ID is derived from the public key.
3. New encodings are created by applying the time-asymmetric SLOTH permutation as `encode(genesis_piece, farmer_id, plot_index)`
4. Each encoding is written directly to disk.
5. A commitment, or tag, to each encoding is created as `hmac(encoding, salt)` and stored within a binary search tree (BST).

This process currently takes ~ 36 hours per TiB on a quad-core machine, but for 1 GiB plotting should take between a few seconds and a few minutes.

### Solving
Once plotting is complete the farmer may join the network and participate in consensus.

1. Connect to a client and subscribe to `slot_notifications` via JSON-RPC.
2. Given a challenge as `hash(epoch_randomness||slot_index) and `SOLUTION_RANGE`.
3. Query the BST for the nearest tag to the challenge.
4. If it within `SOLUTION_RANGE` return a `SOLUTION` else return `None`

## Using Docker
The simplest way to use spartan-farmer is to use container image:
```bash
docker volume create spartan-farmer
docker run --rm -it --mount source=spartan-farmer,target=/var/spartan subspacelabs/spartan-farmer --help
```

`spartan-farmer` is the volume where plot and identity will be stored, it only needs to be created once.

## Install (manually)
Instead of Docker you can also install spartan-farmer natively by compiling it using cargo.

**Notes:** This will currently only work on Mac and Linux, not Windows.

If you have not previously installed the `gmp_mpfr_sys` crate, follow these [instructions](https://docs.rs/gmp-mpfr-sys/1.3.0/gmp_mpfr_sys/index.html#building-on-gnulinux). 

RocksDB on Linux needs LLVM/Clang:
```bash
sudo apt-get install llvm clang
```

Then install the framer using Cargo:
```
cargo +nightly install spartan-farmer
```

NOTE: Above command requires nightly compiler for now, make sure to install it if you don't have one yet:
```
rustup toolchain install nightly
```

## Usage
Commands here assume you installed native binary, but you can also easily adapt them to using with Docker.

Use `--help` to find out all available commands and their options:
```
spartan-farmer --help
```

### Create a New Plot
```
spartan-farmer plot <optional parameters> <piece-count> <seed>
```

This will create a 1 GB plot:
```
spartan-farmer plot 256000 test
```

For all supported options check help:
```
spartan-farmer plot --help
```

By default, plots are written to the OS-specific users local data directory.

```
Linux
$XDG_DATA_HOME or                   /home/alice/.local/share
$HOME/.local/share 

macOS
$HOME/Library/Application Support   /Users/Alice/Library/Application Support

Windows
{FOLDERID_LocalAppData}             C:\Users\Alice\AppData\Local
```

### Start the farmer
```
RUST_LOG=debug spartan-farmer farm
```

This will connect to local node and will try to solve on every slot notification.

*NOTE: You need to have node running before starting farmer, otherwise it will not be able to start*




