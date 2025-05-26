use alloy_consensus::{ReceiptEnvelope, Transaction};
use alloy_network::eip2718::{Decodable2718, Encodable2718};
use alloy_primitives::{address, keccak256, Bytes, B256, U256};
use alloy_rpc_types::{Block, TransactionReceipt};
use alloy_trie::{
    proof::{verify_proof, ProofRetainer},
    root::adjust_index_for_rlp,
    HashBuilder, Nibbles,
};
use anyhow::{Context, Ok, Result};
#[cfg(test)]  //only include in tests
use tokio::fs::File;
#[cfg(test)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MerkleProof {
    root: B256,
    proof: Vec<(Nibbles, Bytes)>,
    target: (Nibbles, Bytes),
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TxAndReceiptInclusionProof {
    block_hash: B256,
    tx_hash: B256,
    tx_index: usize,
    txs_number: usize,
    partial_header: alloy_consensus::Header, //header without (tx_root and receipt_root)
    tx_inclusion_proof: MerkleProof,
    receipt_inclusion_proof: MerkleProof,
}

impl TxAndReceiptInclusionProof {
    pub fn block_hash(&self) -> B256 {
        self.block_hash
    }

    pub fn tx_hash(&self) -> B256 {
        self.tx_hash
    }

    pub fn build(
        block: Block,
        receipts: Vec<TransactionReceipt>,
        tx_index: usize,
    ) -> Result<TxAndReceiptInclusionProof> {
        let mut proof = TxAndReceiptInclusionProof::default();

        let txs_len = block.transactions.len();
        let receipts_len = receipts.len();
        if txs_len != receipts_len {
            return Err(anyhow::anyhow!("txs len not equal to receipts len"));
        }
        let txs = block.transactions.into_transactions_vec();
        let tx_root = block.header.transactions_root;
        let receipt_root = block.header.receipts_root;
        let target_index = adjust_index_for_rlp(tx_index, txs_len);
        let index_nibbles = Nibbles::unpack(alloy_rlp::encode(target_index));

        proof.block_hash = block.header.hash;
        proof.tx_index = tx_index;
        proof.txs_number = txs_len;
        proof.partial_header = block.header.clone().into();
        proof.partial_header.transactions_root = B256::default();
        proof.partial_header.receipts_root = B256::default();

        //build and verify tx inclusion proof
        {
            let retainer = ProofRetainer::new(vec![index_nibbles.clone()]);
            let mut trie = HashBuilder::default().with_proof_retainer(retainer);
            for i in 0..txs_len {
                let index = adjust_index_for_rlp(i, txs_len);
                let index_buffer = alloy_rlp::encode_fixed_size(&index);
                let mut encoded = Vec::new();
                txs[index].clone().into_inner().encode_2718(&mut encoded);
                trie.add_leaf(Nibbles::unpack(&index_buffer), &encoded);
            }

            trie.root();
            let inner = trie.take_proof_nodes().into_nodes_sorted();
            let mut target_value = Vec::new();
            txs[target_index]
                .clone()
                .into_inner()
                .encode_2718(&mut target_value);
            proof.tx_hash = keccak256(target_value.clone().as_slice());
            proof.tx_inclusion_proof = MerkleProof {
                root: tx_root,
                proof: inner,
                target: (index_nibbles.clone(), target_value.clone().into()),
            };

            {
                let tx_inclusion_proof = proof.tx_inclusion_proof.clone();
                assert!(verify_proof(
                    tx_inclusion_proof.root,
                    tx_inclusion_proof.target.0,
                    Some(tx_inclusion_proof.target.1.into()),
                    tx_inclusion_proof.proof.iter().map(|(_, node)| node)
                )
                .is_ok(),);
            }
            println!("tx root verify success");
        }

        //build and verify receipt inclusion proof
        {
            let retainer = ProofRetainer::new(vec![index_nibbles.clone()]);
            let mut trie = HashBuilder::default().with_proof_retainer(retainer);

            for i in 0..receipts_len {
                let index = adjust_index_for_rlp(i, receipts_len);
                let index_buffer = alloy_rlp::encode_fixed_size(&index);
                let mut encoded = Vec::new();
                receipts[index]
                    .clone()
                    .into_inner()
                    .into_primitives_receipt()
                    .encode_2718(&mut encoded);
                trie.add_leaf(Nibbles::unpack(&index_buffer), &encoded);

                if index == target_index && !receipts[index].status() {
                    return Err(anyhow::anyhow!("tx is not success"));
                }
            }

            trie.root();
            let inner = trie.take_proof_nodes().into_nodes_sorted();
            let mut target_value = Vec::new();
            receipts[target_index]
                .clone()
                .into_inner()
                .into_primitives_receipt()
                .encode_2718(&mut target_value);
            proof.receipt_inclusion_proof = MerkleProof {
                root: receipt_root,
                proof: inner,
                target: (index_nibbles.clone(), target_value.clone().into()),
            };
            {
                let receipt_inclusion_proof = proof.receipt_inclusion_proof.clone();

                //verify merkle proof
                assert!(verify_proof(
                    receipt_inclusion_proof.root,
                    receipt_inclusion_proof.target.0,
                    Some(receipt_inclusion_proof.target.1.into()),
                    receipt_inclusion_proof.proof.iter().map(|(_, node)| node)
                )
                .is_ok(),);
                println!("receipt root verify success");
            }
        }

        Ok(proof)
    }

    pub fn verify(&self) -> bool {
        //1. verify tx/receipt inclusion proof
        {
            let proof = self.tx_inclusion_proof.clone();
            if verify_proof(
                proof.root,
                proof.target.0,
                Some(proof.target.1.into()),
                proof.proof.iter().map(|(_, node)| node),
            )
            .is_err()
            {
                return false;
            }

            let proof = self.receipt_inclusion_proof.clone();
            if verify_proof(
                proof.root,
                proof.target.0,
                Some(proof.target.1.into()),
                proof.proof.iter().map(|(_, node)| node),
            )
            .is_err()
            {
                return false;
            }
        }

        //2. same index
        {
            let target_index = adjust_index_for_rlp(self.tx_index, self.txs_number);
            let index_nibbles = Nibbles::unpack(alloy_rlp::encode(&target_index));
            if self.tx_inclusion_proof.target.0 != index_nibbles {
                return false;
            }

            if self.receipt_inclusion_proof.target.0 != index_nibbles {
                return false;
            }
        }

        //3. verify block hash
        {
            let mut header = self.partial_header.clone();
            header.transactions_root = self.tx_inclusion_proof.root;
            header.receipts_root = self.receipt_inclusion_proof.root;

            let rlp_encoded_header = alloy_rlp::encode(&header);
            let calculated_block_hash = alloy_primitives::keccak256(rlp_encoded_header);
            if self.block_hash != calculated_block_hash {
                return false;
            }
        }

        //4 verify tx hash
        {
            let encoded_tx = &self.tx_inclusion_proof.target.1;
            let calculated_tx_hash = alloy_primitives::keccak256(encoded_tx);
            if self.tx_hash != calculated_tx_hash {
                return false;
            }
        }

        //5. check receipt.status = 1
        {
            let binding = self.receipt_inclusion_proof.target.1.clone();
            let mut encoded_receipt = binding.as_ref();
            let receipt = ReceiptEnvelope::decode_2718(&mut encoded_receipt).unwrap();
            if !receipt.is_success() {
                return false;
            }
        }

        true
    }

}

#[cfg(test)]
impl TxAndReceiptInclusionProof {
        pub async fn save_to_json_file(&self, file_path: &str) -> Result<()> {
        let data = serde_json::to_string_pretty(&self)
            .with_context(|| "failed to serialize TxAndReceiptInclusionProof to JSON")?;
        let mut proof_file = File::create(file_path)
            .await
            .with_context(|| format!("failed to create file: {}", file_path))?;
        proof_file
            .write_all(data.as_bytes())
            .await
            .with_context(|| format!("failed to write JSON to file: {}", file_path))?;
        Ok(())
    }

    pub async fn load_from_json_file(file_path: &str) -> Result<Self> {
        let mut file = File::open(file_path)
            .await
            .with_context(|| format!("failed to open file: {}", file_path))?;
        let mut data = String::new();
        file.read_to_string(&mut data)
            .await
            .with_context(|| format!("failed to read file: {}", file_path))?;
        let proof: TxAndReceiptInclusionProof = serde_json::from_str(&data)
            .context("failed to deserialize JSON into TxAndReceiptInclusionProof")?;
        Ok(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloy_network::Ethereum;
    use alloy_provider::{Provider, RootProvider};

    #[test]
    fn test_verify_proof() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let file_pathes =  [
            "testdata/tx_0x0a4f541f32887ea04a1d855ab7ee8248782ffaac9004259f5e4362e7ec860a94.proof",
            "testdata/tx_0x0a61443b776b7a7a3114591e65a651bc10b9f9511dabe454393d6482fe8eee58.proof"
            ];

            for file_path in file_pathes {
                let proof = TxAndReceiptInclusionProof::load_from_json_file(file_path)
                    .await
                    .unwrap();
                assert!(proof.verify());
            }
        })
    }

    #[test]
    fn test_build_proof() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let provider = RootProvider::<Ethereum>::new_http(
                "https://ethereum.rpc.subquery.network/public"
                    .parse()
                    .unwrap(),
            );

            // Get the latest block number
            let block_number = 22545789;
            let block = provider
                .get_block(block_number.into())
                .full()
                .await
                .unwrap()
                .unwrap();
            let receipts = provider
                .get_block_receipts(block_number.into())
                .await
                .unwrap()
                .unwrap();

            for (i, receipt) in receipts.clone().iter().enumerate() {
                if receipt.status() {
                    let _ = TxAndReceiptInclusionProof::build(block.clone(), receipts.clone(), i);
                }
            }
        })
    }
}
