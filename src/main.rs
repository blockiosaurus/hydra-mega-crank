use anchor_client::{Client, Cluster, Program};
use anchor_lang::{system_program, AccountDeserialize};
use anchor_spl::associated_token::get_associated_token_address;
use anyhow::Result;
use mpl_hydra::state::{
    Fanout, FanoutMembershipMintVoucher, FanoutMembershipVoucher, FanoutMint, MembershipModel,
    FANOUT_MEMBERSHIP_VOUCHER_SIZE, FANOUT_MINT_MEMBERSHIP_VOUCHER_SIZE,
};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    pubkey,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature, Signer},
    sysvar::rent,
    transaction::Transaction,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use std::{env, rc::Rc};

const HYDRA_ADDRESS: Pubkey = pubkey!("hyDQ4Nz1eYyegS6JfenyKwKzYxRsCWCriYSAjtzP4Vg");

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: cargo run <SOLANA_RPC_URL> <KEYPAIR_PATH>");
        return;
    }

    let rpc_url = args[1].clone();
    let connection = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let (client, keypair) = setup_client(args[1].clone(), args[2].clone()).unwrap();
    let program = client.program(HYDRA_ADDRESS);

    top_down(&connection, &program, &keypair);
}

pub fn setup_client(rpc_url: String, keypair_path: String) -> Result<(Client, Keypair)> {
    let ws_url = rpc_url.replace("http", "ws");
    let cluster = Cluster::Custom(rpc_url, ws_url);

    let keypair = read_keypair_file(keypair_path);
    match keypair {
        Ok(keypair) => {
            let key_bytes = keypair.to_bytes();
            let signer = Rc::new(Keypair::from_bytes(&key_bytes)?);

            let opts = CommitmentConfig::confirmed();
            Ok((Client::new_with_options(cluster, signer, opts), keypair))
        }
        Err(_) => {
            println!("Unable to read keypair file");
            return Err(anyhow::anyhow!("Unable to read keypair file"));
        }
    }
}

fn bottom_up(connection: &RpcClient, program: &Program) {
    // #[allow(deprecated)]
    let mint_voucher_accounts = connection
        .get_program_accounts_with_config(
            &HYDRA_ADDRESS,
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::DataSize(
                    FANOUT_MINT_MEMBERSHIP_VOUCHER_SIZE.try_into().unwrap(),
                )]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(connection.commitment()),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .unwrap();

    println!("Num Mint Vouchers: {}", mint_voucher_accounts.len());
    for (_, account) in mint_voucher_accounts.iter().enumerate() {
        let voucher =
            FanoutMembershipMintVoucher::try_deserialize(&mut account.1.data.as_slice()).unwrap();
        println!(
                "Mint Voucher: FanoutMembershipMintVoucher {{\n    fanout: {:?},\n    fanout_mint: {:?},\n    last_inflow: {:?},\n    bump_seed: {:?},\n}}",
                voucher.fanout,
                voucher.fanout_mint,
                voucher.last_inflow,
                voucher.bump_seed,
            );
    }
}

fn top_down(connection: &RpcClient, program: &Program, keypair: &Keypair) {
    #[allow(deprecated)]
    let accounts = connection
        .get_program_accounts_with_config(
            &HYDRA_ADDRESS,
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::DataSize(300)]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(connection.commitment()),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .unwrap();

    for (_, account) in accounts.iter().enumerate() {
        println!("{:?}", account.0);
        let fanout = match Fanout::try_deserialize(&mut account.1.data.as_slice()) {
            Ok(fanout) => fanout,
            Err(_) => continue,
        };
        println!("Fanout:\n{:?}\n{:#?}", account.0, fanout);

        // #[allow(deprecated)]
        // let mint_voucher_accounts = connection
        //     .get_program_accounts_with_config(
        //         &HYDRA_ADDRESS,
        //         RpcProgramAccountsConfig {
        //             filters: Some(vec![
        //                 RpcFilterType::DataSize(
        //                     FANOUT_MINT_MEMBERSHIP_VOUCHER_SIZE.try_into().unwrap(),
        //                 ),
        //                 RpcFilterType::Memcmp(Memcmp {
        //                     offset: 8,
        //                     bytes: MemcmpEncodedBytes::Bytes(account.0.to_bytes().to_vec()),
        //                     encoding: None,
        //                 }),
        //             ]),
        //             account_config: RpcAccountInfoConfig {
        //                 encoding: Some(UiAccountEncoding::Base64),
        //                 commitment: Some(connection.commitment()),
        //                 ..RpcAccountInfoConfig::default()
        //             },
        //             ..RpcProgramAccountsConfig::default()
        //         },
        //     )
        //     .unwrap();

        // for (_, account) in mint_voucher_accounts.iter().enumerate() {
        //     let voucher =
        //         FanoutMembershipMintVoucher::try_deserialize(&mut account.1.data.as_slice())
        //             .unwrap();
        //     println!(
        //         "Mint Voucher: FanoutMembershipMintVoucher {{\n    fanout: {:?},\n    fanout_mint: {:?},\n    last_inflow: {:?},\n    bump_seed: {:?},\n}}",
        //         voucher.fanout,
        //         voucher.fanout_mint,
        //         voucher.last_inflow,
        //         voucher.bump_seed,
        //     );
        // }

        #[allow(deprecated)]
        let fanout_mint_accounts = connection
            .get_program_accounts_with_config(
                &HYDRA_ADDRESS,
                RpcProgramAccountsConfig {
                    filters: Some(vec![
                        RpcFilterType::DataSize(200),
                        RpcFilterType::Memcmp(Memcmp {
                            offset: 40,
                            bytes: MemcmpEncodedBytes::Bytes(account.0.to_bytes().to_vec()),
                            encoding: None,
                        }),
                    ]),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        commitment: Some(connection.commitment()),
                        ..RpcAccountInfoConfig::default()
                    },
                    ..RpcProgramAccountsConfig::default()
                },
            )
            .unwrap();

        for (_, fanout_mint_account) in fanout_mint_accounts.iter().enumerate() {
            let fanout_mint =
                FanoutMint::try_deserialize(&mut fanout_mint_account.1.data.as_slice()).unwrap();
            println!(
                "Fanout Mint:\n{:?}\n{:#?}",
                fanout_mint_account.0, fanout_mint
            );

            #[allow(deprecated)]
            let voucher_accounts = connection
                .get_program_accounts_with_config(
                    &HYDRA_ADDRESS,
                    RpcProgramAccountsConfig {
                        filters: Some(vec![
                            RpcFilterType::DataSize(
                                FANOUT_MEMBERSHIP_VOUCHER_SIZE.try_into().unwrap(),
                            ),
                            RpcFilterType::Memcmp(Memcmp {
                                offset: 8,
                                bytes: MemcmpEncodedBytes::Bytes(account.0.to_bytes().to_vec()),
                                encoding: None,
                            }),
                        ]),
                        account_config: RpcAccountInfoConfig {
                            encoding: Some(UiAccountEncoding::Base64),
                            commitment: Some(connection.commitment()),
                            ..RpcAccountInfoConfig::default()
                        },
                        ..RpcProgramAccountsConfig::default()
                    },
                )
                .unwrap();

            for (_, voucher_account) in voucher_accounts.iter().enumerate() {
                let voucher = FanoutMembershipVoucher::try_deserialize(
                    &mut voucher_account.1.data.as_slice(),
                )
                .unwrap();
                println!("Voucher: {:#?}", voucher);

                let mint_voucher_address = Pubkey::find_program_address(
                    &[
                        "fanout-membership".as_bytes(),
                        fanout_mint_account.0.as_ref(),
                        voucher.membership_key.as_ref(),
                        fanout_mint.mint.as_ref(),
                    ],
                    &mpl_hydra::ID,
                );

                let (member_mint_ata, member_stake_ata) = match fanout.membership_mint {
                    Some(mint) => {
                        let ata = get_associated_token_address(&voucher.membership_key, &mint);
                        match connection.get_account(&ata) {
                            Ok(_) => {
                                println!("Member Mint ATA Exists: {:?}", ata);
                            }
                            Err(err) => {
                                let ix = create_associated_token_account(
                                    &program.payer(),
                                    &voucher.membership_key,
                                    &mint,
                                );
                                let tx = Transaction::new_signed_with_payer(
                                    &[ix],
                                    Some(&keypair.pubkey()),
                                    &[keypair],
                                    connection.get_latest_blockhash().unwrap(),
                                );
                                let signature =
                                    connection.send_and_confirm_transaction(&tx).unwrap();
                                println!("Create Member Mint ATA: {:?}", err);
                            }
                        }
                        let stake_ata = get_associated_token_address(&voucher_account.0, &mint);
                        (Some(ata), Some(stake_ata))
                    }
                    None => {
                        println!("Fanout has no membership Mint");
                        (None, None)
                    }
                };

                let fanout_mint_member_ata =
                    get_associated_token_address(&voucher.membership_key, &fanout_mint.mint);

                match build(
                    program,
                    &fanout,
                    account,
                    &fanout_mint,
                    fanout_mint_account,
                    &voucher,
                    voucher_account,
                    &mint_voucher_address.0,
                    &fanout_mint_member_ata,
                    &member_mint_ata,
                    &member_stake_ata,
                ) {
                    Ok(signature) => {
                        println!("Signature: {:?}", signature);
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    program: &Program,
    fanout: &Fanout,
    account: &(Pubkey, Account),
    fanout_mint: &FanoutMint,
    fanout_mint_account: &(Pubkey, Account),
    voucher: &FanoutMembershipVoucher,
    voucher_account: &(Pubkey, Account),
    mint_voucher_address: &Pubkey,
    fanout_mint_member_ata: &Pubkey,
    member_mint_ata: &Option<Pubkey>,
    member_stake_ata: &Option<Pubkey>,
) -> Result<Signature> {
    let builder = match fanout.membership_model {
        MembershipModel::Wallet => program
            .request()
            .accounts(mpl_hydra::accounts::DistributeWalletMember {
                payer: program.payer(),
                member: voucher.membership_key,
                membership_voucher: voucher_account.0,
                fanout: account.0,
                holding_account: fanout_mint.token_account,
                fanout_for_mint: fanout_mint_account.0,
                fanout_for_mint_membership_voucher: *mint_voucher_address,
                fanout_mint: fanout_mint.mint,
                fanout_mint_member_token_account: *fanout_mint_member_ata,
                system_program: system_program::ID,
                rent: rent::ID,
                token_program: spl_token::ID,
            })
            .args(mpl_hydra::instruction::ProcessDistributeWallet {
                distribute_for_mint: true,
            }),
        MembershipModel::Token => program
            .request()
            .accounts(mpl_hydra::accounts::DistributeTokenMember {
                payer: program.payer(),
                member: voucher.membership_key,
                membership_mint_token_account: member_mint_ata.unwrap_or_default(),
                membership_voucher: voucher_account.0,
                fanout: account.0,
                holding_account: fanout_mint.token_account,
                fanout_for_mint: fanout_mint_account.0,
                fanout_for_mint_membership_voucher: *mint_voucher_address,
                fanout_mint: fanout_mint.mint,
                fanout_mint_member_token_account: *fanout_mint_member_ata,
                system_program: system_program::ID,
                rent: rent::ID,
                token_program: spl_token::ID,
                membership_mint: fanout.membership_mint.unwrap(),
                member_stake_account: member_stake_ata.unwrap_or_default(),
            })
            .args(mpl_hydra::instruction::ProcessDistributeToken {
                distribute_for_mint: true,
            }),
        MembershipModel::NFT => program
            .request()
            .accounts(mpl_hydra::accounts::DistributeNftMember {
                payer: program.payer(),
                member: voucher.membership_key,
                membership_mint_token_account: member_mint_ata.unwrap_or_default(),
                membership_key: voucher.membership_key,
                membership_voucher: voucher_account.0,
                fanout: account.0,
                holding_account: fanout_mint.token_account,
                fanout_for_mint: fanout_mint_account.0,
                fanout_for_mint_membership_voucher: *mint_voucher_address,
                fanout_mint: fanout_mint.mint,
                fanout_mint_member_token_account: *fanout_mint_member_ata,
                system_program: system_program::ID,
                rent: rent::ID,
                token_program: spl_token::ID,
            })
            .args(mpl_hydra::instruction::ProcessDistributeNft {
                distribute_for_mint: true,
            }),
    };
    let ix = builder.instructions();
    match builder.send_with_spinner_and_config(RpcSendTransactionConfig {
        skip_preflight: true,
        ..RpcSendTransactionConfig::default()
    }) {
        Ok(signature) => {
            println!("Signature: {:?}", signature);
            Ok(signature)
        }
        Err(err) => {
            println!("Error: {:?}", err);
            Err(err.into())
        }
    }
}
