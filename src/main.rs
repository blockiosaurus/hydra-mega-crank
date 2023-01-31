use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType},
};
use solana_sdk::{commitment_config::CommitmentConfig, program_pack::Pack, pubkey, pubkey::Pubkey};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run <SOLANA_RPC_URL>");
        return;
    }

    const TRIFLE_ADDRESS: Pubkey = pubkey!("trifMWutwBxkSuatmpPVnEe7NoE3BJKgjVi8sSyoXWX");

    let rpc_url = args[1].clone();
    let connection = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let filters = Some(vec![RpcFilterType::Memcmp(Memcmp {
        offset: 0,
        bytes: MemcmpEncodedBytes::Bytes(vec![0x01]),
        encoding: None,
    })]);

    let accounts = connection
        .get_program_accounts_with_config(
            &TRIFLE_ADDRESS,
            RpcProgramAccountsConfig {
                filters,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(connection.commitment()),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .unwrap();

    let mut total_balance = 0.0;
    for (i, account) in accounts.iter().enumerate() {
        total_balance += (account.1.lamports as f64) * 1e-9;
        println!(
            "-- Account Address {:?}:  {:?} has key {:?} and {:?} SOL --",
            i,
            account.0,
            account.1.data[0],
            (account.1.lamports as f64) * 1e-9
        );
    }

    println!("Total Constraint Models: {:?}", accounts.len());
    println!("Total balance: {:?} SOL", total_balance);
}
