#![cfg(test)]

use super::*;
use access_control::{role_admin, role_oracle, role_settlement_operator};
use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger as _},
    token, Address, BytesN, Env, String, Symbol,
};

fn setup_payment_processor(env: &Env) -> (Address, PaymentProcessorClient<'_>) {
    let contract_id = env.register(PaymentProcessor, ());
    let client = PaymentProcessorClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize_payment_processor(&admin);
    (admin, client)
}

fn setup_refund_manager(env: &Env) -> (Address, RefundManagerClient<'_>) {
    let contract_id = env.register(RefundManager, ());
    let client = RefundManagerClient::new(env, &contract_id);
    let admin = Address::generate(env);

    let token_admin = Address::generate(env);
    let usdc_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.initialize_refund_manager(&admin, &usdc_token);

    let token_admin_client = token::StellarAssetClient::new(env, &usdc_token);
    token_admin_client.mint(&contract_id, &1_000_000_000_000i128);

    (admin, client)
}

#[test]
fn test_create_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "payment_123");
    let merchant_id = Address::generate(&env);
    let amount = 1000000000i128; // 1000 USDC (6 decimals)
    let currency = Symbol::new(&env, "USDC");
    let deposit_address = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let payment = client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &currency,
        &deposit_address,
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    assert_eq!(payment.payment_id, payment_id);
    assert_eq!(payment.merchant_id, merchant_id);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.currency, currency);
    assert_eq!(payment.deposit_address, deposit_address);
    assert_eq!(payment.status, PaymentStatus::Pending);
    assert_eq!(payment.memo, None);
    assert_eq!(payment.memo_type, None);
}

#[test]
fn test_create_payment_rate_limit_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let currency = Symbol::new(&env, "USDC");
    let deposit_address = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 3600;

    for i in 0..CREATE_PAYMENT_MAX_PER_WINDOW {
        let payment_id = format_id(&env, "rate_limit_", i as u64);
        client.create_payment(
            &payment_id,
            &merchant_id,
            &100i128,
            &currency,
            &deposit_address,
            &Some(expires_at),
                        &None::<u64>,
                        &None::<String>,
            &None::<String>,
            &None::<Address>,
        );
    }

    let overflow_id = String::from_str(&env, "rate_limit_overflow");
    let overflow = client.try_create_payment(
        &overflow_id,
        &merchant_id,
        &100i128,
        &currency,
        &deposit_address,
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    assert_eq!(overflow, Err(Ok(Error::RateLimitExceeded)));
}

#[test]
fn test_verify_payment_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "payment_123");
    let merchant_id = Address::generate(&env);
    let amount = 1000000000i128;
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    let payer_address = Address::generate(&env);
    let transaction_hash = BytesN::<32>::random(&env);
    let oracle = Address::generate(&env);
    client.grant_role(&admin, &role_oracle(&env), &oracle);

    let status = client.verify_payment(
        &oracle,
        &payment_id,
        &transaction_hash,
        &payer_address,
        &amount,
    );

    assert_eq!(status, PaymentStatus::Confirmed);
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Confirmed);
    assert_eq!(payment.amount_received, Some(amount));
}

#[test]
fn test_verify_payment_partially_paid() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "partial_pay");
    let merchant_id = Address::generate(&env);
    let amount = 1000000000i128;
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    let oracle = Address::generate(&env);
    client.grant_role(&admin, &role_oracle(&env), &oracle);

    // Send significantly less than expected (outside tolerance)
    let amount_received = amount - 100;
    let status = client.verify_payment(
        &oracle,
        &payment_id,
        &BytesN::<32>::random(&env),
        &Address::generate(&env),
        &amount_received,
    );

    assert_eq!(status, PaymentStatus::PartiallyPaid);
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::PartiallyPaid);
    assert_eq!(payment.amount_received, Some(amount_received));
}

#[test]
fn test_verify_payment_overpaid() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "over_pay");
    let merchant_id = Address::generate(&env);
    let amount = 1000000000i128;
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    let oracle = Address::generate(&env);
    client.grant_role(&admin, &role_oracle(&env), &oracle);

    // Send more than expected (outside tolerance)
    let amount_received = amount + 100;
    let status = client.verify_payment(
        &oracle,
        &payment_id,
        &BytesN::<32>::random(&env),
        &Address::generate(&env),
        &amount_received,
    );

    assert_eq!(status, PaymentStatus::Overpaid);
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Overpaid);
    assert_eq!(payment.amount_received, Some(amount_received));
}

#[test]
fn test_verify_payment_within_tolerance() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "tol_pay");
    let merchant_id = Address::generate(&env);
    let amount = 1000000000i128;
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    let oracle = Address::generate(&env);
    client.grant_role(&admin, &role_oracle(&env), &oracle);

    // Send exactly 1 stroop less — within tolerance → Confirmed
    let amount_received = amount - 1;
    let status = client.verify_payment(
        &oracle,
        &payment_id,
        &BytesN::<32>::random(&env),
        &Address::generate(&env),
        &amount_received,
    );

    assert_eq!(status, PaymentStatus::Confirmed);
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Confirmed);
    assert_eq!(payment.amount_received, Some(amount_received));
}

#[test]
fn test_get_merchant_payments_index_and_pagination() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    let currency = Symbol::new(&env, "USDC");
    let deposit_address = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 3600;

    let payment_id_1 = String::from_str(&env, "merchant_pay_1");
    let payment_id_2 = String::from_str(&env, "merchant_pay_2");
    let payment_id_3 = String::from_str(&env, "merchant_pay_3");

    client.grant_role(&admin, &role_merchant(&env), &merchant_id);
    client.create_payment(
        &payment_id_1,
        &merchant_id,
        &100i128,
        &currency,
        &deposit_address,
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    client.create_payment(
        &payment_id_2,
        &merchant_id,
        &200i128,
        &currency,
        &deposit_address,
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    client.create_payment(
        &payment_id_3,
        &merchant_id,
        &300i128,
        &currency,
        &deposit_address,
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    let all = client.get_merchant_payments(&merchant_id);
    assert_eq!(all.len(), 3);
    assert_eq!(all.get(0), Some(payment_id_1.clone()));
    assert_eq!(all.get(1), Some(payment_id_2.clone()));
    assert_eq!(all.get(2), Some(payment_id_3.clone()));

    let page = client.get_merchant_payments_paginated(&merchant_id, &1u32, &2u32);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0), Some(payment_id_2));
    assert_eq!(page.get(1), Some(payment_id_3));
}

#[test]
fn test_cancel_pending_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "cancel_pending_success");
    let merchant_id = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &500i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    // Set time to before expiry
    env.ledger().set_timestamp(expires_at - 1);

    client.cancel_payment(&merchant_id, &payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Failed);

    let events = env.events().all();
    let last_event = events.last().unwrap();
    let topics = last_event.1;
    assert_eq!(topics.get(0).unwrap(), Symbol::new(&env, "PAYMENT").into_val(&env));
    assert_eq!(topics.get(1).unwrap(), Symbol::new(&env, "CANCELLED").into_val(&env));
    assert_eq!(topics.get(2).unwrap(), payment_id.into_val(&env));
}

#[test]
fn test_cancel_fails_when_confirmed() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "cancel_fails_confirmed");
    let merchant_id = Address::generate(&env);
    let amount = 500i128;
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    let oracle = Address::generate(&env);
    client.grant_role(&admin, &role_oracle(&env), &oracle);

    client.verify_payment(
        &oracle,
        &payment_id,
        &BytesN::<32>::random(&env),
        &Address::generate(&env),
        &amount,
    );

    let res = client.try_cancel_payment(&merchant_id, &payment_id);
    assert_eq!(res.unwrap_err().unwrap(), Error::PaymentAlreadyProcessed);
}

#[test]
fn test_expiry_logic() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "cancel_past_expiry");
    let merchant_id = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &500i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    // Set time to past expiry
    env.ledger().set_timestamp(expires_at + 1);

    // This should correctly mark it Expired, not throw an error
    let res = client.try_cancel_payment(&merchant_id, &payment_id);
    assert!(res.is_ok());

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Expired);

    let events = env.events().all();
    let last_event = events.last().unwrap();
    let topics = last_event.1;
    assert_eq!(topics.get(0).unwrap(), Symbol::new(&env, "PAYMENT").into_val(&env));
    assert_eq!(topics.get(1).unwrap(), Symbol::new(&env, "EXPIRED").into_val(&env));
    assert_eq!(topics.get(2).unwrap(), payment_id.into_val(&env));
}

#[test]
fn test_unauthorized_cancel() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "unauth_cancel");
    let merchant_id = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 3600;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &500i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    let random_addr = Address::generate(&env);
    let res = client.try_cancel_payment(&random_addr, &payment_id);
    assert_eq!(res.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_expire_payment_after_deadline() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "expire_after_deadline");
    let merchant_id = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 10;
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.create_payment(
        &payment_id,
        &merchant_id,
        &500i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    env.ledger().set_timestamp(expires_at + 1);
    client.expire_payment(&payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Expired);
}

#[test]
fn test_create_and_get_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_123");
    let merchant_id = Address::generate(&env);
    let refund_amount = 1000i128;
    let reason = String::from_str(&env, "Reason");
    let requester = Address::generate(&env);

    // Register payment so refund amount can be validated
    client.register_payment(
        &payment_id,
        &merchant_id,
        &5000i128,
        &Symbol::new(&env, "USDC"),
    );

    let refund_id = client.create_refund(&payment_id, &refund_amount, &reason, &requester);
    let refund = client.get_refund(&refund_id);

    assert_eq!(refund.payment_id, payment_id);
    assert_eq!(refund.amount, refund_amount);
    assert_eq!(refund.status, RefundStatus::Pending);
}

#[test]
fn test_process_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_123");
    let merchant_id = Address::generate(&env);
    let refund_amount = 1000i128;
    let requester = Address::generate(&env);

    client.register_payment(
        &payment_id,
        &merchant_id,
        &5000i128,
        &Symbol::new(&env, "USDC"),
    );

    let refund_id = client.create_refund(
        &payment_id,
        &refund_amount,
        &String::from_str(&env, "Reason"),
        &requester,
    );

    let operator = Address::generate(&env);
    client.grant_role(&admin, &role_settlement_operator(&env), &operator);

    client.process_refund(&operator, &refund_id);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Completed);
}

#[test]
fn test_initialize_contract() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let usdc_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let contract_id = env.register(RefundManager, ());
    let client = RefundManagerClient::new(&env, &contract_id);
    client.initialize_refund_manager(&admin, &usdc_token);

    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert!(client.has_role(&role_admin(&env), &admin));
}

#[test]
fn test_grant_role() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_refund_manager(&env);
    let account = Address::generate(&env);
    let role = role_oracle(&env);

    client.grant_role(&admin, &role, &account);
    assert!(client.has_role(&role, &account));
}

#[test]
fn test_transfer_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (current_admin, client) = setup_refund_manager(&env);
    let new_admin = Address::generate(&env);

    client.transfer_admin(&current_admin, &new_admin);
    assert!(client.has_role(&role_admin(&env), &new_admin));
    assert_eq!(client.get_admin(), Some(new_admin));
}

#[test]
fn test_multiple_refunds_unique_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_123");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);

    client.register_payment(
        &payment_id,
        &merchant_id,
        &5000i128,
        &Symbol::new(&env, "USDC"),
    );

    // Create first refund
    let refund_id_1 = client.create_refund(
        &payment_id,
        &1000i128,
        &String::from_str(&env, "First refund"),
        &requester,
    );

    // Create second refund
    let refund_id_2 = client.create_refund(
        &payment_id,
        &500i128,
        &String::from_str(&env, "Second refund"),
        &requester,
    );

    // Create third refund
    let refund_id_3 = client.create_refund(
        &payment_id,
        &250i128,
        &String::from_str(&env, "Third refund"),
        &requester,
    );

    // Verify all refund IDs are unique
    assert_ne!(refund_id_1, refund_id_2);
    assert_ne!(refund_id_2, refund_id_3);
    assert_ne!(refund_id_1, refund_id_3);

    // Verify all refunds can be retrieved independently
    let refund_1 = client.get_refund(&refund_id_1);
    let refund_2 = client.get_refund(&refund_id_2);
    let refund_3 = client.get_refund(&refund_id_3);

    assert_eq!(refund_1.amount, 1000i128);
    assert_eq!(refund_2.amount, 500i128);
    assert_eq!(refund_3.amount, 250i128);

    // Verify refund IDs follow expected pattern
    assert_eq!(refund_id_1, String::from_str(&env, "refund_1"));
    assert_eq!(refund_id_2, String::from_str(&env, "refund_2"));
    assert_eq!(refund_id_3, String::from_str(&env, "refund_3"));
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn test_create_refund_requires_auth() {
    let env = Env::default();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_123");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);

    client.register_payment(
        &payment_id,
        &merchant_id,
        &5000i128,
        &Symbol::new(&env, "USDC"),
    );

    // This should panic because we're not mocking auth
    client.create_refund(
        &payment_id,
        &1000i128,
        &String::from_str(&env, "Unauthorized refund"),
        &requester,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn test_create_payment_requires_auth() {
    let env = Env::default();
    let (_admin, client) = setup_payment_processor(&env);

    let payment_id = String::from_str(&env, "payment_123");
    let merchant_id = Address::generate(&env);
    let amount = 1000000000i128;
    let currency = Symbol::new(&env, "USDC");
    let deposit_address = Address::generate(&env);
    let expires_at = env.ledger().timestamp() + 3600;

    // This should panic because we're not mocking auth
    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &currency,
        &deposit_address,
        &Some(expires_at),
                &None::<u64>,
                &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
}

/// Issue #37: verify role membership list integrity.
#[test]
fn test_get_role_members() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_refund_manager(&env);

    let oracle1 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let oracle_role = role_oracle(&env);

    // Initially no oracle members
    let members = client.get_role_members(&oracle_role);
    assert_eq!(members.len(), 0);

    // Grant oracle to oracle1
    client.grant_role(&admin, &oracle_role, &oracle1);
    let members = client.get_role_members(&oracle_role);
    assert_eq!(members.len(), 1);
    assert_eq!(members.get(0), Some(oracle1.clone()));

    // Grant oracle to oracle2
    client.grant_role(&admin, &oracle_role, &oracle2);
    let members = client.get_role_members(&oracle_role);
    assert_eq!(members.len(), 2);

    // Revoke oracle1 — list should shrink
    client.revoke_role(&admin, &oracle_role, &oracle1);
    let members = client.get_role_members(&oracle_role);
    assert_eq!(members.len(), 1);
    assert_eq!(members.get(0), Some(oracle2.clone()));
}

/// Issue #37: admin is automatically in the ADMIN role members list after initialize.
#[test]
fn test_admin_in_role_members_after_init() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_refund_manager(&env);

    let admin_role = role_admin(&env);
    let members = client.get_role_members(&admin_role);
    assert_eq!(members.len(), 1);
    assert_eq!(members.get(0), Some(admin));
}

fn setup_refund_manager_with_token(env: &Env) -> (Address, RefundManagerClient<'_>, Address) {
    let contract_id = env.register(RefundManager, ());
    let client = RefundManagerClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let usdc_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.initialize_refund_manager(&admin, &usdc_token);
    let token_admin_client = token::StellarAssetClient::new(env, &usdc_token);
    token_admin_client.mint(&contract_id, &1_000_000_000_000i128);
    (admin, client, usdc_token)
}

#[test]
fn test_process_refund_deducts_fee_from_requester() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client, usdc_token) = setup_refund_manager_with_token(&env);

    let payment_id = String::from_str(&env, "payment_fee_1");
    let merchant_id = Address::generate(&env);
    let refund_amount = 10_000i128;
    let requester = Address::generate(&env);

    client.register_payment(&payment_id, &merchant_id, &refund_amount, &Symbol::new(&env, "USDC"));
    let refund_id = client.create_refund(&payment_id, &refund_amount, &String::from_str(&env, "fee test"), &requester);

    let operator = Address::generate(&env);
    client.grant_role(&admin, &role_settlement_operator(&env), &operator);
    client.process_refund(&operator, &refund_id);

    let token_client = token::TokenClient::new(&env, &usdc_token);
    let fee = refund_amount * 100 / 10_000; // 1%
    let net = refund_amount - fee;

    assert_eq!(token_client.balance(&requester), net);
}

#[test]
fn test_process_refund_sends_fee_to_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client, usdc_token) = setup_refund_manager_with_token(&env);

    let payment_id = String::from_str(&env, "payment_fee_2");
    let merchant_id = Address::generate(&env);
    let refund_amount = 10_000i128;
    let requester = Address::generate(&env);

    client.register_payment(&payment_id, &merchant_id, &refund_amount, &Symbol::new(&env, "USDC"));
    let refund_id = client.create_refund(&payment_id, &refund_amount, &String::from_str(&env, "fee test"), &requester);
#[test]
fn test_cancel_refund_by_requester() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_cancel_1");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);

    client.register_payment(&payment_id, &merchant_id, &5000i128, &Symbol::new(&env, "USDC"));
    let refund_id = client.create_refund(&payment_id, &1000i128, &String::from_str(&env, "cancel me"), &requester);

    client.cancel_refund(&requester, &refund_id);

    // Refund record should be gone
    let result = client.try_get_refund(&refund_id);
    assert_eq!(result, Err(Ok(Error::RefundNotFound)));

    // Payment refund list should be empty
    let refunds = client.get_payment_refunds(&payment_id).unwrap();
    assert_eq!(refunds.len(), 0);
}

#[test]
fn test_cancel_refund_by_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_cancel_2");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);

    client.register_payment(&payment_id, &merchant_id, &5000i128, &Symbol::new(&env, "USDC"));
    let refund_id = client.create_refund(&payment_id, &500i128, &String::from_str(&env, "admin cancel"), &requester);

    client.cancel_refund(&admin, &refund_id);

    let result = client.try_get_refund(&refund_id);
    assert_eq!(result, Err(Ok(Error::RefundNotFound)));
}

#[test]
fn test_cancel_refund_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_cancel_3");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);

    client.register_payment(&payment_id, &merchant_id, &5000i128, &Symbol::new(&env, "USDC"));
    let refund_id = client.create_refund(&payment_id, &500i128, &String::from_str(&env, "reason"), &requester);

    let random = Address::generate(&env);
    let result = client.try_cancel_refund(&random, &refund_id);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_cancel_refund_already_processed() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_cancel_4");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);

    client.register_payment(&payment_id, &merchant_id, &5000i128, &Symbol::new(&env, "USDC"));
    let refund_id = client.create_refund(&payment_id, &500i128, &String::from_str(&env, "reason"), &requester);

    let operator = Address::generate(&env);
    client.grant_role(&admin, &role_settlement_operator(&env), &operator);
    client.process_refund(&operator, &refund_id);

    let token_client = token::TokenClient::new(&env, &usdc_token);
    let fee = refund_amount * 100 / 10_000; // 1%

    assert_eq!(token_client.balance(&admin), fee);
    // Attempt to cancel a completed refund
    let result = client.try_cancel_refund(&requester, &refund_id);
    assert_eq!(result, Err(Ok(Error::RefundAlreadyProcessed)));
}

#[test]
fn test_cancel_refund_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "payment_cancel_5");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);

    client.register_payment(&payment_id, &merchant_id, &5000i128, &Symbol::new(&env, "USDC"));
    let refund_id = client.create_refund(&payment_id, &750i128, &String::from_str(&env, "reason"), &requester);

    client.cancel_refund(&requester, &refund_id);

    let events = env.events().all();
    let last = events.last().unwrap();
    let topics = last.1;
    assert_eq!(topics.get(0).unwrap(), Symbol::new(&env, "REFUND").into_val(&env));
    assert_eq!(topics.get(1).unwrap(), Symbol::new(&env, "CANCELLED").into_val(&env));
}

// --- Payment expiry / duration tests ---

#[test]
fn test_create_payment_with_explicit_expires_at() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let expires_at = env.ledger().timestamp() + 7200; // 2 hours
    let payment = client.create_payment(
        &String::from_str(&env, "pay_explicit_expiry"),
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(expires_at),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(payment.expires_at, expires_at);
}

#[test]
fn test_create_payment_with_duration_secs() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let now = env.ledger().timestamp();
    let duration = 1800u64; // 30 minutes
    let payment = client.create_payment(
        &String::from_str(&env, "pay_duration"),
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &None::<u64>,
        &Some(duration),
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(payment.expires_at, now + duration);
}

#[test]
fn test_create_payment_defaults_to_one_hour() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let now = env.ledger().timestamp();
    let payment = client.create_payment(
        &String::from_str(&env, "pay_default_expiry"),
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &None::<u64>,
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(payment.expires_at, now + DEFAULT_PAYMENT_DURATION_SECS);
}

#[test]
fn test_create_payment_explicit_expires_at_overrides_duration() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let explicit_ts = env.ledger().timestamp() + 9999;
    let payment = client.create_payment(
        &String::from_str(&env, "pay_explicit_wins"),
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(explicit_ts),
        &Some(60u64), // duration ignored when expires_at is Some
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(payment.expires_at, explicit_ts);
}

#[test]
fn test_create_payment_past_expires_at_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let now = env.ledger().timestamp();
    // expires_at in the past (or equal to now)
    let result = client.try_create_payment(
        &String::from_str(&env, "pay_past_expiry"),
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(now), // now is not > now, so invalid
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(result, Err(Ok(Error::InvalidExpiry)));
}

#[test]
fn test_create_payment_zero_duration_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let result = client.try_create_payment(
        &String::from_str(&env, "pay_zero_duration"),
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &None::<u64>,
        &Some(0u64), // 0 seconds → expires_at == now → invalid
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(result, Err(Ok(Error::InvalidExpiry)));
}

// --- Amount limits tests ---

#[test]
fn test_global_min_limit_blocks_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    client.set_global_amount_limits(&admin, &Some(500i128), &None::<i128>);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let result = client.try_create_payment(
        &String::from_str(&env, "pay_below_global_min"),
        &merchant_id,
        &499i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(result, Err(Ok(Error::AmountBelowMin)));
}

#[test]
fn test_global_max_limit_blocks_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    client.set_global_amount_limits(&admin, &None::<i128>, &Some(1000i128));

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let result = client.try_create_payment(
        &String::from_str(&env, "pay_above_global_max"),
        &merchant_id,
        &1001i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(result, Err(Ok(Error::AmountAboveMax)));
}

#[test]
fn test_global_limits_allow_payment_within_range() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    client.set_global_amount_limits(&admin, &Some(100i128), &Some(10_000i128));

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let payment = client.create_payment(
        &String::from_str(&env, "pay_within_global"),
        &merchant_id,
        &5_000i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(payment.status, PaymentStatus::Pending);
}

#[test]
fn test_merchant_limits_override_global_limits() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    // Global: min 1000
    client.set_global_amount_limits(&admin, &Some(1000i128), &None::<i128>);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    // Merchant-specific: min 10 (lower than global)
    client.set_merchant_amount_limits(&merchant_id, &Some(10i128), &None::<i128>);

    // 500 is below global min but above merchant min — should succeed
    let payment = client.create_payment(
        &String::from_str(&env, "pay_merchant_override"),
        &merchant_id,
        &500i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(payment.status, PaymentStatus::Pending);
}

#[test]
fn test_merchant_max_limit_blocks_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    client.set_merchant_amount_limits(&merchant_id, &None::<i128>, &Some(200i128));

    let result = client.try_create_payment(
        &String::from_str(&env, "pay_above_merchant_max"),
        &merchant_id,
        &201i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );
    assert_eq!(result, Err(Ok(Error::AmountAboveMax)));
}

#[test]
fn test_set_merchant_limits_invalid_range_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    // min > max — must fail
    let result = client.try_set_merchant_amount_limits(
        &merchant_id,
        &Some(1000i128),
        &Some(500i128),
    );
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_get_merchant_and_global_limits() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    assert_eq!(client.get_global_amount_limits(), None);
    assert_eq!(client.get_merchant_amount_limits(&merchant_id), None);

    client.set_global_amount_limits(&admin, &Some(50i128), &Some(5000i128));
    client.set_merchant_amount_limits(&merchant_id, &Some(100i128), &Some(2000i128));

    let global = client.get_global_amount_limits().unwrap();
    assert_eq!(global.min, Some(50i128));
    assert_eq!(global.max, Some(5000i128));

    let merchant = client.get_merchant_amount_limits(&merchant_id).unwrap();
    assert_eq!(merchant.min, Some(100i128));
    assert_eq!(merchant.max, Some(2000i128));
}

// --- Multi-asset payment tests ---

#[test]
fn test_create_payment_with_allowed_token() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let token_admin = Address::generate(&env);
    let alt_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    // Allow the token
    client.allow_token(&admin, &alt_token);
    assert!(client.is_token_allowed(&alt_token));

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let payment_id = String::from_str(&env, "pay_alt_token");
    let payment = client.create_payment(
        &payment_id,
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "EURC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &Some(alt_token.clone()),
    );

    assert_eq!(payment.token_address, Some(alt_token));
    assert_eq!(payment.status, PaymentStatus::Pending);
}

#[test]
fn test_create_payment_with_unlisted_token_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let token_admin = Address::generate(&env);
    let unknown_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    // Do NOT allow the token
    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let result = client.try_create_payment(
        &String::from_str(&env, "pay_bad_token"),
        &merchant_id,
        &1000i128,
        &Symbol::new(&env, "RAND"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &Some(unknown_token),
    );

    assert_eq!(result, Err(Ok(Error::UnsupportedToken)));
}

#[test]
fn test_create_payment_no_token_address_uses_default() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let merchant_id = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);

    let payment = client.create_payment(
        &String::from_str(&env, "pay_default_token"),
        &merchant_id,
        &500i128,
        &Symbol::new(&env, "USDC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &None::<Address>,
    );

    assert_eq!(payment.token_address, None);
    assert_eq!(payment.status, PaymentStatus::Pending);
}

#[test]
fn test_verify_payment_decimal_aware_tolerance_7_decimals() {
    // A token with 7 decimals should have tolerance = 10 (10^(7-6))
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let token_admin = Address::generate(&env);
    let alt_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    // Stellar asset contracts report 7 decimals
    client.allow_token(&admin, &alt_token);

    let merchant_id = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);
    client.grant_role(&admin, &role_oracle(&env), &oracle);

    let payment_id = String::from_str(&env, "pay_7dec");
    let amount = 1_000_000_0i128; // 1.0 in 7-decimal units
    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &Symbol::new(&env, "EURC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &Some(alt_token),
    );

    // Underpay by 10 (within 7-decimal tolerance of 10) → Confirmed
    let status = client.verify_payment(
        &oracle,
        &payment_id,
        &BytesN::<32>::random(&env),
        &Address::generate(&env),
        &(amount - 10),
    );
    assert_eq!(status, PaymentStatus::Confirmed);
}

#[test]
fn test_verify_payment_decimal_aware_tolerance_7_decimals_overpay() {
    // Underpay by 11 (outside 7-decimal tolerance of 10) → PartiallyPaid
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_payment_processor(&env);

    let token_admin = Address::generate(&env);
    let alt_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.allow_token(&admin, &alt_token);

    let merchant_id = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.grant_role(&admin, &role_merchant(&env), &merchant_id);
    client.grant_role(&admin, &role_oracle(&env), &oracle);

    let payment_id = String::from_str(&env, "pay_7dec_partial");
    let amount = 1_000_000_0i128;
    client.create_payment(
        &payment_id,
        &merchant_id,
        &amount,
        &Symbol::new(&env, "EURC"),
        &Address::generate(&env),
        &Some(env.ledger().timestamp() + 3600),
        &None::<u64>,
        &None::<String>,
        &None::<String>,
        &Some(alt_token),
    );

    // Underpay by 11 → PartiallyPaid
    let status = client.verify_payment(
        &oracle,
        &payment_id,
        &BytesN::<32>::random(&env),
        &Address::generate(&env),
        &(amount - 11),
    );
    assert_eq!(status, PaymentStatus::PartiallyPaid);
}

// --- Cumulative refund cap tests ---

#[test]
fn test_cumulative_refunds_exceed_payment_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "pay_cumulative_1");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);
    let payment_amount = 1000i128;

    client.register_payment(&payment_id, &merchant_id, &payment_amount, &Symbol::new(&env, "USDC"));

    // First refund: 600 — ok
    client.create_refund(&payment_id, &600i128, &String::from_str(&env, "partial 1"), &requester);

    // Second refund: 500 — 600 + 500 = 1100 > 1000 — must fail
    let result = client.try_create_refund(
        &payment_id,
        &500i128,
        &String::from_str(&env, "partial 2"),
        &requester,
    );
    assert_eq!(result, Err(Ok(Error::RefundExceedsPayment)));
}

#[test]
fn test_refund_exactly_equal_to_payment_amount_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "pay_exact_1");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);
    let payment_amount = 1000i128;

    client.register_payment(&payment_id, &merchant_id, &payment_amount, &Symbol::new(&env, "USDC"));

    // Single refund equal to full payment amount — must succeed
    let refund_id = client.create_refund(
        &payment_id,
        &payment_amount,
        &String::from_str(&env, "full refund"),
        &requester,
    );
    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.amount, payment_amount);
    assert_eq!(refund.status, RefundStatus::Pending);
}

#[test]
fn test_second_refund_after_full_refund_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "pay_full_then_extra");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);
    let payment_amount = 1000i128;

    client.register_payment(&payment_id, &merchant_id, &payment_amount, &Symbol::new(&env, "USDC"));

    // Full refund — ok
    client.create_refund(&payment_id, &payment_amount, &String::from_str(&env, "full"), &requester);

    // Any additional refund — must fail
    let result = client.try_create_refund(
        &payment_id,
        &1i128,
        &String::from_str(&env, "extra"),
        &requester,
    );
    assert_eq!(result, Err(Ok(Error::RefundExceedsPayment)));
}

#[test]
fn test_rejected_refunds_not_counted_in_cumulative_total() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup_refund_manager(&env);

    let payment_id = String::from_str(&env, "pay_rejected_refund");
    let merchant_id = Address::generate(&env);
    let requester = Address::generate(&env);
    let payment_amount = 1000i128;

    client.register_payment(&payment_id, &merchant_id, &payment_amount, &Symbol::new(&env, "USDC"));

    // Create and reject a refund for 800
    let refund_id = client.create_refund(
        &payment_id,
        &800i128,
        &String::from_str(&env, "will be rejected"),
        &requester,
    );
    let operator = Address::generate(&env);
    client.grant_role(&admin, &role_settlement_operator(&env), &operator);
    client.reject_refund(&operator, &refund_id);

    // A new refund for 1000 should succeed because the rejected one is excluded
    let new_refund_id = client.create_refund(
        &payment_id,
        &payment_amount,
        &String::from_str(&env, "after rejection"),
        &requester,
    );
    let refund = client.get_refund(&new_refund_id);
    assert_eq!(refund.amount, payment_amount);
    assert_eq!(refund.status, RefundStatus::Pending);
}
