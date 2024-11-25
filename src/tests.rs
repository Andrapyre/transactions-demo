use crate::engine::AccountStore;
use crate::models::{Account, Transaction, TransactionType};
use rust_decimal::Decimal;

fn run_test(transactions: Vec<Transaction>, expected_output: Account) {
    let mut store = AccountStore::new();

    transactions.into_iter().for_each(|tx| {
        let _ = store.add_tx(tx);
    });

    let actual_outputs = store.export_accounts();
    assert_eq!(actual_outputs[0], expected_output)
}

fn test_dispute_tx_with_invalid_id(transaction_type: TransactionType) {
    let amount = Decimal::new(5432, 4);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(transaction_type, 1, 2, None),
    ];
    run_test(
        txs,
        Account::new(1, amount.clone(), Decimal::new(0, 1), amount.clone(), false),
    )
}

fn test_dispute_resolution_without_dispute(transaction_type: TransactionType) {
    let amount = Decimal::new(5432, 4);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(transaction_type, 1, 1, None),
    ];
    run_test(
        txs,
        Account::new(1, amount.clone(), Decimal::new(0, 1), amount.clone(), false),
    )
}

fn test_tx_with_locked_account(tx: Transaction) {
    let amount = Decimal::new(5432, 4);
    let deposit = Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone()));
    let txs = vec![
        deposit.clone(),
        deposit,
        Transaction::new(TransactionType::Dispute, 1, 1, None),
        Transaction::new(TransactionType::Chargeback, 1, 1, None),
        tx,
    ];
    run_test(
        txs,
        Account::new(1, amount.clone(), Decimal::new(0, 1), amount.clone(), true),
    )
}

#[test]
fn basic_deposit_should_succeed() {
    let amount = Decimal::new(5432, 4);
    let txs = vec![Transaction::new(
        TransactionType::Deposit,
        1,
        1,
        Some(amount.clone()),
    )];
    run_test(
        txs,
        Account::new(1, amount.clone(), Decimal::new(0, 1), amount.clone(), false),
    )
}

#[test]
fn basic_withdrawal_should_succeed() {
    let amount = Decimal::new(5432, 4);
    let leftover = Decimal::new(2191, 4);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(
            TransactionType::Withdrawal,
            1,
            2,
            Some(Decimal::new(3241, 4)),
        ),
    ];
    run_test(
        txs,
        Account::new(
            1,
            leftover.clone(),
            Decimal::new(0, 1),
            leftover.clone(),
            false,
        ),
    )
}

#[test]
fn basic_dispute_should_succeed() {
    let amount = Decimal::new(5432, 4);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(TransactionType::Dispute, 1, 1, None),
    ];
    run_test(
        txs,
        Account::new(1, Decimal::new(0, 1), amount.clone(), amount.clone(), false),
    )
}

#[test]
fn basic_chargeback_should_succeed() {
    let amount = Decimal::new(5432, 4);
    let zero = Decimal::new(0, 1);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(TransactionType::Dispute, 1, 1, None),
        Transaction::new(TransactionType::Chargeback, 1, 1, None),
    ];
    run_test(
        txs,
        Account::new(1, zero.clone(), zero.clone(), zero.clone(), true),
    )
}

#[test]
fn basic_resolve_should_succeed() {
    let amount = Decimal::new(5432, 4);
    let zero = Decimal::new(0, 1);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(TransactionType::Dispute, 1, 1, None),
        Transaction::new(TransactionType::Resolve, 1, 1, None),
    ];
    run_test(
        txs,
        Account::new(1, amount.clone(), zero.clone(), amount.clone(), false),
    )
}

#[test]
fn overdraw_should_fail() {
    let amount = Decimal::new(5432, 4);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(
            TransactionType::Withdrawal,
            1,
            2,
            Some(Decimal::new(5433, 4)),
        ),
    ];
    run_test(
        txs,
        Account::new(1, amount.clone(), Decimal::new(0, 1), amount, false),
    )
}

#[test]
fn dispute_resulting_in_hold_overdraw_should_fail() {
    let amount = Decimal::new(5432, 4);
    let final_amount = Decimal::new(2877, 4);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone())),
        Transaction::new(
            TransactionType::Withdrawal,
            1,
            2,
            Some(Decimal::new(2555, 4)),
        ),
        Transaction::new(TransactionType::Dispute, 1, 1, None),
    ];
    run_test(
        txs,
        Account::new(
            1,
            final_amount.clone(),
            Decimal::new(0, 1),
            final_amount,
            false,
        ),
    )
}

#[test]
fn dispute_with_invalid_id_should_fail() {
    test_dispute_tx_with_invalid_id(TransactionType::Dispute)
}

#[test]
fn chargeback_with_invalid_id_should_fail() {
    test_dispute_tx_with_invalid_id(TransactionType::Chargeback)
}

#[test]
fn resolve_with_invalid_id_should_fail() {
    test_dispute_tx_with_invalid_id(TransactionType::Resolve)
}

#[test]
fn resolve_without_dispute_should_do_nothing() {
    test_dispute_resolution_without_dispute(TransactionType::Resolve)
}

#[test]
fn chargeback_without_dispute_should_do_nothing() {
    test_dispute_resolution_without_dispute(TransactionType::Chargeback)
}

#[test]
fn deposit_to_locked_account_should_fail() {
    test_tx_with_locked_account(Transaction::new(
        TransactionType::Deposit,
        1,
        5,
        Some(Decimal::new(54, 0)),
    ))
}

#[test]
fn withdrawal_from_locked_account_should_fail() {
    test_tx_with_locked_account(Transaction::new(
        TransactionType::Withdrawal,
        1,
        5,
        Some(Decimal::new(54, 0)),
    ))
}

#[test]
fn dispute_on_locked_account_should_fail() {
    test_tx_with_locked_account(Transaction::new(TransactionType::Dispute, 1, 2, None))
}
