# SP1 Project Template

This is a template for creating an end-to-end [SP1](https://github.com/succinctlabs/sp1) project
that can generate a proof of any RISC-V program.

## Requirements

- [Rust](https://rustup.rs/)
- [SP1](https://docs.succinct.xyz/docs/sp1/getting-started/install)

### Generate an SP1 Core Proof

To generate an SP1 [core proof](https://docs.succinct.xyz/docs/sp1/generating-proofs/proof-types#core-default) for your program:

```sh
cd script
RUST_LOG=info cargo run --release -- --rpc-url https://ethereum.rpc.subquery.network/public --tx-hash 0x0a4f541f32887ea04a1d855ab7ee8248782ffaac9004259f5e4362e7ec860a94
```

### Retrieve the Verification Key

To retrieve your `programVKey` for your on-chain contract, run the following command in `script`:

```sh
cd script
cargo run --release --bin vkey
```


