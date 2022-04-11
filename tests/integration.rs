#![cfg(feature = "test-bpf")]

use sol_escrow::{
    entrypoint::process_instruction,
    state::{EscrowPDA, OfferData},
};
use solana_program_test::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    signer::{keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::state::Account;
use std::convert::TryInto;

const MINT_SIZE: u64 = 82;

struct PartyKeys {
    main: keypair::Keypair,
    mint_1: Pubkey,
    mint_2: Pubkey,
}

struct TestEnv {
    program_key: keypair::Keypair,
    buyer: PartyKeys,
    seller: PartyKeys,
    seller_temp: keypair::Keypair,
    mint_1: keypair::Keypair,
    mint_2: keypair::Keypair,
    ctx: ProgramTestContext,
}

#[tokio::test]
async fn test_offer_accept() {
    let mut env = init_env().await;
    let pda = create_offer(&mut env).await;
    accept_offer(&mut env, pda).await;
}

#[tokio::test]
async fn test_offer_cancel() {
    let mut env = init_env().await;
    let pda = create_offer(&mut env).await;
    cancel_offer(&mut env, pda, true).await;
}

#[tokio::test]
async fn test_cancel_recreate() {
    let mut env = init_env().await;
    let pda = create_offer(&mut env).await;
    cancel_offer(&mut env, pda, false).await;
    env.ctx.warp_to_slot(10).unwrap();
    let pda = create_offer(&mut env).await;
    accept_offer(&mut env, pda).await;
}

async fn init_env() -> TestEnv {
    let program_key = keypair::Keypair::new();
    let buyer = keypair::Keypair::new();
    let seller = keypair::Keypair::new();
    let seller_temp = keypair::Keypair::new();
    let mint_1 = keypair::Keypair::new();
    let mint_2 = keypair::Keypair::new();
    let mut ctx = ProgramTest::new(
        "sol_escrow",
        program_key.pubkey(),
        processor!(process_instruction),
    )
    .start_with_context()
    .await;

    let min_rent = Rent::default().minimum_balance(MINT_SIZE as usize);

    let c1 = system_instruction::create_account(
        &ctx.payer.pubkey(),
        &mint_1.pubkey(),
        min_rent,
        MINT_SIZE,
        &spl_token::id(),
    );
    let c2 = system_instruction::create_account(
        &ctx.payer.pubkey(),
        &mint_2.pubkey(),
        min_rent,
        MINT_SIZE,
        &spl_token::id(),
    );

    let i1 = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_1.pubkey(),
        &ctx.payer.pubkey(),
        None,
        1,
    )
    .expect("could not create initialise_mint instruction");

    let i2 = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_2.pubkey(),
        &ctx.payer.pubkey(),
        None,
        1,
    )
    .expect("could not create initialise_mint instruction");

    let tx = Transaction::new_signed_with_payer(
        &[c1],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_1],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[c2],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &mint_2],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[i1],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[i2],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let split_txs = system_instruction::transfer_many(
        &ctx.payer.pubkey(),
        &[(buyer.pubkey(), 100000000), (seller.pubkey(), 100000000)],
    );
    let tx = Transaction::new_signed_with_payer(
        &split_txs[0..2],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let b1 =
        create_associated_token_account(&ctx.payer.pubkey(), &buyer.pubkey(), &mint_1.pubkey());
    let b2 =
        create_associated_token_account(&ctx.payer.pubkey(), &buyer.pubkey(), &mint_2.pubkey());
    let b3 =
        create_associated_token_account(&ctx.payer.pubkey(), &seller.pubkey(), &mint_1.pubkey());
    let b4 =
        create_associated_token_account(&ctx.payer.pubkey(), &seller.pubkey(), &mint_2.pubkey());

    let tx = Transaction::new_signed_with_payer(
        &[b1, b2, b3, b4],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let b1 = get_associated_token_address(&buyer.pubkey(), &mint_1.pubkey());
    let b2 = get_associated_token_address(&buyer.pubkey(), &mint_2.pubkey());
    let s1 = get_associated_token_address(&seller.pubkey(), &mint_1.pubkey());
    let s2 = get_associated_token_address(&seller.pubkey(), &mint_2.pubkey());

    let min_rent = Rent::default().minimum_balance(165);

    let st = system_instruction::create_account(
        &seller.pubkey(),
        &seller_temp.pubkey(),
        min_rent,
        165,
        &spl_token::id(),
    );

    let sti = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &seller_temp.pubkey(),
        &mint_1.pubkey(),
        &seller.pubkey(),
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[st, sti],
        Some(&ctx.payer.pubkey()),
        &[&seller_temp, &seller, &ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let asset_mint = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_1.pubkey(),
        &seller_temp.pubkey(),
        &ctx.payer.pubkey(),
        &[&ctx.payer.pubkey()],
        5,
    )
    .unwrap();

    let strike_mint = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_2.pubkey(),
        &b2,
        &ctx.payer.pubkey(),
        &[&ctx.payer.pubkey()],
        5,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[asset_mint, strike_mint],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.unwrap();

    let buyer = PartyKeys {
        main: buyer,
        mint_1: b1,
        mint_2: b2,
    };

    let seller = PartyKeys {
        main: seller,
        mint_1: s1,
        mint_2: s2,
    };

    TestEnv {
        buyer,
        seller,
        mint_1,
        mint_2,
        seller_temp,
        program_key,
        ctx,
    }
}

async fn create_offer(test_env: &mut TestEnv) -> EscrowPDA {
    let offer_data = OfferData {
        token_type: test_env.mint_1.pubkey(),
        token_qty: 5,
        strike_type: test_env.mint_2.pubkey(),
        strike_qty: 5,
    };

    let (pda, _bump) =
        Pubkey::find_program_address(&[&offer_data.get_seed()], &test_env.program_key.pubkey());

    let accounts = vec![
        AccountMeta {
            pubkey: test_env.seller.main.pubkey(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.seller_temp.pubkey(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.seller.mint_2.clone(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: pda,
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: system_program::id(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: spl_token::id(),
            is_signer: false,
            is_writable: false,
        },
    ];

    let mut ixd = Vec::with_capacity(81);
    ixd.push(0);
    ixd.extend_from_slice(&offer_data.to_bytes());
    let instruction_data: [u8; 81] = ixd.try_into().unwrap();

    println!("sending create_offer transaction");
    let ix =
        Instruction::new_with_bytes(test_env.program_key.pubkey(), &instruction_data, accounts);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&test_env.ctx.payer.pubkey()),
        &[&test_env.ctx.payer, &test_env.seller.main],
        test_env.ctx.last_blockhash,
    );
    test_env
        .ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap();

    let expected_data = EscrowPDA {
        offer_data,
        seller_main: test_env.seller.main.pubkey(),
        seller_temp: test_env.seller_temp.pubkey(),
        seller_receive: test_env.seller.mint_2.clone(),
    };

    let pda_acc = test_env
        .ctx
        .banks_client
        .get_account(pda)
        .await
        .unwrap()
        .expect("could not find escrow_pda account");
    let pda_data = EscrowPDA::unpack_from_slice(&pda_acc.data[..]).unwrap();

    println!("asserting escrow PDA state");
    assert_eq!(expected_data, pda_data, "incorrect escrow PDA data");

    println!("trying illegal transaction");
    let ix = spl_token::instruction::transfer(
        &spl_token::id(),
        &test_env.seller_temp.pubkey(),
        &test_env.buyer.mint_1,
        &test_env.seller.main.pubkey(),
        &[&test_env.seller.main.pubkey()],
        5,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&test_env.ctx.payer.pubkey()),
        &[&test_env.ctx.payer, &test_env.seller.main],
        test_env.ctx.last_blockhash,
    );
    test_env
        .ctx
        .banks_client
        .process_transaction(tx)
        .await
        .expect_err("illegal transaction did not fail");

    expected_data
}

async fn get_token_balance(test_env: &mut TestEnv, pk: Pubkey) -> u64 {
    let acc = test_env
        .ctx
        .banks_client
        .get_account(pk)
        .await
        .unwrap()
        .unwrap();
    Account::unpack_from_slice(&acc.data[..]).unwrap().amount
}

async fn accept_offer(test_env: &mut TestEnv, escrow_pda: EscrowPDA) {
    let seed = escrow_pda.offer_data.get_seed();
    let (pda, _bump) = Pubkey::find_program_address(&[&seed], &test_env.program_key.pubkey());

    let buyer_init_balance = get_token_balance(test_env, test_env.buyer.mint_1.clone()).await;
    let seller_init_balance = get_token_balance(test_env, test_env.seller.mint_2.clone()).await;

    let accounts = vec![
        AccountMeta {
            pubkey: test_env.buyer.main.pubkey(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.buyer.mint_2.clone(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.buyer.mint_1.clone(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: pda,
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.seller.main.pubkey(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.seller_temp.pubkey(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.seller.mint_2.clone(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: system_program::id(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: spl_token::id(),
            is_signer: false,
            is_writable: false,
        },
    ];

    let instruction_data = &[1];

    println!("sending accept_offer transaction");
    let ix = Instruction::new_with_bytes(test_env.program_key.pubkey(), instruction_data, accounts);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&test_env.ctx.payer.pubkey()),
        &[&test_env.ctx.payer, &test_env.buyer.main],
        test_env.ctx.last_blockhash,
    );
    test_env
        .ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap();

    test_env
        .ctx
        .banks_client
        .get_account(pda)
        .await
        .unwrap()
        .ok_or("")
        .expect_err("PDA account not closed");
    println!("PDA account closed");

    let buyer_balance = get_token_balance(test_env, test_env.buyer.mint_1.clone()).await;
    let seller_balance = get_token_balance(test_env, test_env.seller.mint_2.clone()).await;

    println!("asserting transfer states");

    assert_eq!(
        buyer_balance - buyer_init_balance,
        escrow_pda.offer_data.token_qty,
        "incorrect token qty transferred to buyer"
    );
    assert_eq!(
        seller_balance - seller_init_balance,
        escrow_pda.offer_data.strike_qty,
        "incorrect strike qty transferred to seller"
    );
}

async fn cancel_offer(test_env: &mut TestEnv, escrow_pda: EscrowPDA, test_close: bool) {
    let seed = escrow_pda.offer_data.get_seed();
    let (pda, _bump) = Pubkey::find_program_address(&[&seed], &test_env.program_key.pubkey());

    let accounts = vec![
        AccountMeta {
            pubkey: test_env.seller.main.pubkey(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.seller_temp.pubkey(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: test_env.seller.mint_2.clone(),
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: pda,
            is_signer: false,
            is_writable: true,
        },
        AccountMeta {
            pubkey: system_program::id(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: spl_token::id(),
            is_signer: false,
            is_writable: false,
        },
    ];

    let instruction_data = &[2];

    println!("sending cancel_offer transaction");
    let ix = Instruction::new_with_bytes(test_env.program_key.pubkey(), instruction_data, accounts);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&test_env.ctx.payer.pubkey()),
        &[&test_env.ctx.payer, &test_env.seller.main],
        test_env.ctx.last_blockhash,
    );
    test_env
        .ctx
        .banks_client
        .process_transaction(tx)
        .await
        .unwrap();

    test_env
        .ctx
        .banks_client
        .get_account(pda)
        .await
        .unwrap()
        .ok_or("")
        .expect_err("PDA account not closed");
    println!("PDA account closed");

    if test_close {
        println!("testing transfer from seller_temp");
        let ix = spl_token::instruction::transfer(
            &spl_token::id(),
            &test_env.seller_temp.pubkey(),
            &test_env.seller.mint_1.clone(),
            &test_env.seller.main.pubkey(),
            &[&test_env.seller.main.pubkey()],
            5,
        )
        .unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&test_env.ctx.payer.pubkey()),
            &[&test_env.ctx.payer, &test_env.seller.main],
            test_env.ctx.last_blockhash,
        );
        test_env
            .ctx
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();
        println!("closing seller_temp");
        let ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &test_env.seller_temp.pubkey(),
            &test_env.seller.main.pubkey(),
            &test_env.seller.main.pubkey(),
            &[&test_env.seller.main.pubkey()],
        )
        .unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&test_env.ctx.payer.pubkey()),
            &[&test_env.ctx.payer, &test_env.seller.main],
            test_env.ctx.last_blockhash,
        );
        test_env
            .ctx
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap();
    }
}
