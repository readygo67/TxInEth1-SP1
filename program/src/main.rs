//! A simple program that takes a number `n` as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use tx_in_eth1_lib::{TxAndReceiptInclusionProof};

pub fn main() {
    let proof = sp1_zkvm::io::read::<TxAndReceiptInclusionProof>();
    proof.verify();
    
    let mut public = proof.block_hash().to_vec();
    public.extend_from_slice(proof.tx_hash().as_ref());

    sp1_zkvm::io::commit_slice(&public);
}

