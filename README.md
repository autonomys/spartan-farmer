<div align="center">
  <h1><code>spartan-farmer</code></h1>
  <strong>A proof-of-concept farmer for the <a href="https://subspace.network/">Subspace Network Blockchain</a></strong>
</div>

## Overview

**Notes:** The code is un-audited and not production ready, use it at your own risk.

Subspace is a proof-of-storage blockchain that resolves the farmer's dilemma, to learn more read our <a href="https://drive.google.com/file/d/1v847u_XeVf0SBz7Y7LEMXi72QfqirstL/view">whitepaper</a>. 

This a bare-bones farmer for the simplified Spartan proof-of-space variant of Subspace. A farmer is similar to a miner in a proof-of-work blockchain, but instead of wasting CPU cycles, it wastes disk space. Much of this code has been extracted from <a href="https://www.github.com/subspace/subspace-core-rust">subspace-core-rust</a>.

The farmer basically has two modes: plotting and farming.

### Plotting

1. A genesis piece is created from a short seed.
2. A new Schnorr key pair is generated, and the farmer ID is derived from the public key.
3. New encodings are created by applying the time-asymmetric SLOTH permutation as `encode(genesis_piece, farmer_id, plot_index)`
4. Each encoding is written directly to disk.
5. A commitment, or tag, to each encoding is created as `hmac(encoding, salt)` and stored within a binary search tree (BST).

This process currently takes ~ 36 hours per TB on a quad-core machine.

### Solving

Once plotting is complete the farmer may join the network and participate in consensus.

1. Connect to a client and subscribe to `slot_notifications` via JSON-RPC.
2. Given a challenge as `hash(epoch_randomness||slot_index) and `SOLUTION_RANGE`.
3. Query the BST for the nearest tag to the challenge.
4. If it within `SOLUTION_RANGE` return a `SOLUTION` else return `None`

## Install

**Notes:** This will currently only work on Mac and Linux, not Windows.

If you have not previously installed the `gmp_mpfr_sys` crate, follow these [instructions](https://docs.rs/gmp-mpfr-sys/1.3.0/gmp_mpfr_sys/index.html#building-on-gnulinux). 

RocksDB on Linux needs LLVM/Clang:
```bash
sudo apt-get install llvm clang
```

```
git clone https://github.com/subspace/spartan-farmer.git
cd spartan-farmer
cargo build --release
```

## Usage

### Create a New Plot

`cargo run plotter <optional path> <piece-count> <seed>`

Creates a 1 GB plot

`cargo run plotter 256000 test`

By default, plots are written to the users local data directory.

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

`RUST_LOG=debug cargo run -- farm`

**Notes:** You must delete the existing plot before creating a new one.





