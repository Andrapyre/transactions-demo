use rust_decimal::Decimal;
use crate::engine::AccountStore;
use crate::models::{Account, Transaction, TransactionType};

fn run_test(transactions: Vec<Transaction>, expected_output: Account) {
    let mut store = AccountStore::new();

    transactions.into_iter().for_each(|tx| {
        let _ = store.add_tx(tx);
    });

    let actual_outputs = store.export_accounts();
    assert_eq!(actual_outputs[0], expected_output)
}

#[test]
fn basic_deposit() {
    let amount = Decimal::new(5432, 4);
    let txs = vec![
        Transaction::new(TransactionType::Deposit, 1, 1, Some(amount.clone()))
    ];
    run_test(txs, Account::new(1, amount.clone(), Decimal::new(0, 1), amount.clone(), false))
}