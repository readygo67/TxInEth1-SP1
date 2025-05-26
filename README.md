# TxInEth1-SP1 

This repo proves a successfully executed transaction included in a block on SP1.


### Basic idea
TxAndReceiptInclusionProof is core data structure, which include
- block_hash,  
- tx_hash,
- partical_header without tx_root and receipt_root,
- 2 merkle proofs(tx_inclusion_proof and receipt_inclusion_proof)

```rust
pub struct MerkleProof {
    root: B256,
    proof: Vec<(Nibbles, Bytes)>,
    target: (Nibbles, Bytes),
}

pub struct TxAndReceiptInclusionProof {
    block_hash: B256,
    tx_hash: B256,
    tx_index: usize,
    txs_number: usize,
    partial_header: alloy_consensus::Header, //header without (tx_root and receipt_root)
    tx_inclusion_proof: MerkleProof,
    receipt_inclusion_proof: MerkleProof,
}
```
TxAndReceiptInclusionProof's verify() method verify
- can rebuild merkle root from data and merkle path for tx_inclusion_proof and receipt_inclusion_proof
- tx_inclusion_proof and receipt_inclusion_proof have same index
- keccak(rlp(tx))= tx_hash
- block_hash = keccak256(rlp(partical_header + tx_inclusion_proof.root + receipt_inclusion_proof.root))
- receipt.status = 1,(hence, can not build proof for failed transaction)

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


