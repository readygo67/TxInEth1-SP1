use alloy_network::Ethereum;
use alloy_primitives::{Address, TxHash, B256};
use alloy_provider::{Provider, RootProvider};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin};

use clap::Parser;
use tx_in_eth1_lib::TxAndReceiptInclusionProof;

pub const ELF: &[u8] = include_elf!("tx-in-eth1-program");

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct InputArgs {
    #[arg(short, long)]
    rpc_url: String,

    #[arg(short, long)]
    tx_hash: String,
}

#[tokio::main]
async fn main() {
    sp1_sdk::utils::setup_logger();

    let args = InputArgs{
        rpc_url: "https://ethereum.rpc.subquery.network/public".to_string(),
        tx_hash: "0x0a4f541f32887ea04a1d855ab7ee8248782ffaac9004259f5e4362e7ec860a94".to_string(),
    };
    println!("rpc_url: {}", args.rpc_url);
    println!("tx_hash: {}", args.tx_hash);

    let tx_hash: TxHash = args.tx_hash.parse().unwrap();
    let provider = RootProvider::<Ethereum>::new_http(args.rpc_url.parse().unwrap());
    let tx = provider
        .get_transaction_by_hash(tx_hash)
        .await
        .unwrap()
        .unwrap();
    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await
        .unwrap()
        .unwrap();

    if !receipt.status() {
        println!("tx failed");
        return;
    }

    let block_number = receipt.block_number.unwrap();
    let tx_index = receipt.transaction_index.unwrap();
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

    let data = TxAndReceiptInclusionProof::build(block, receipts, tx_index as usize).unwrap();

    let mut stdin = SP1Stdin::new();
    stdin.write(&data);

    tracing::info!("initializing prover");
    let client = ProverClient::from_env();

    let (pk, vk) = client.setup(ELF);

    let proof = client
        .prove(&pk, &stdin)
        .run()
        .expect("fail to generate proof");

    client.verify(&proof, &vk).expect("fail to verify proof");
    println!("successfully generated proof");
}

