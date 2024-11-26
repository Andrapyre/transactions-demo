use csv::Writer;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::io::Stdout;
use std::{env, io};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;

use crate::models::TransactionState::{ChargedBack, Disputed, Success};
use crate::models::{
    Account, ApplicationError, ApplicationResult, HistoricalTransaction, Transaction,
    TransactionState, TransactionType,
};

fn get_raw_input_file_path() -> ApplicationResult<String> {
    let args: Vec<String> = env::args().collect();
    let length = args.len();

    match env::args_os().nth(1) {
        Some(_) if length > 2 => ApplicationError::err("Multiple file name arguments provided. The transaction processor only accepts one file."),
        Some(filename) if length == 2 => Ok(filename.to_string_lossy().to_string()),
        _ => ApplicationError::err("Input file required in order to process transactions")
    }
}

fn read_records_into_store(
    account_store: &mut AccountStore,
    bytes: &Vec<u8>,
) -> ApplicationResult<()> {
    let mut rdr = csv::ReaderBuilder::new().from_reader(bytes.as_slice());
    for result in rdr.deserialize() {
        let tx: Transaction = result?;
        let _ = account_store.add_tx(tx);
    }

    Ok(())
}

pub async fn process_file() -> ApplicationResult<()> {
    let raw_file_path = get_raw_input_file_path()?;
    let mut store = AccountStore::new();

    let file = tokio::fs::File::open(raw_file_path).await?;
    let reader = BufReader::new(file);

    let stream = reader.lines();
    let mut lines = LinesStream::new(stream);
    let mut header = lines.next().await.unwrap()?;
    header.push("\n".parse().unwrap());
    let endline_char = "\n".as_bytes();
    let mut bytes: Vec<u8> = header.as_bytes().to_vec();
    let mut count_in_chunk = 0;
    let chunk_size = 100;

    while let Some(Ok(line)) = lines.next().await {
        if count_in_chunk < chunk_size {
            bytes.extend_from_slice(&*[endline_char, line.as_bytes()].concat());
            count_in_chunk += 1;
        } else {
            read_records_into_store(&mut store, &bytes)?;
            bytes = header.as_bytes().to_vec();
            count_in_chunk = 0;
        }
    }

    if count_in_chunk > 0 {
        read_records_into_store(&mut store, &bytes)?;
    }

    let mut wtr = Writer::from_writer(io::stdout());
    store.write_output(&mut wtr)?;

    Ok(())
}

#[derive(Clone)]
pub struct InternalAccount {
    held: Decimal,
    total: Decimal,
    is_locked: bool,
}

impl InternalAccount {
    fn to_account(&self, id: u16) -> Account {
        Account::new(
            id,
            (self.total - self.held).round_dp(4),
            self.held,
            self.total,
            self.is_locked,
        )
    }

    fn check_is_locked(&self) -> ApplicationResult<()> {
        if self.is_locked {
            ApplicationError::err("Account is locked")
        } else {
            Ok(())
        }
    }
    fn new_with_no_funds() -> Self {
        InternalAccount::new(Decimal::new(0, 1))
    }
    fn new(amount: Decimal) -> Self {
        Self {
            held: Decimal::new(0, 1),
            total: amount,
            is_locked: false,
        }
    }
    fn withdraw(self, amount: Decimal, is_chargeback: bool) -> ApplicationResult<InternalAccount> {
        let new_total = (self.total - amount).round_dp(4);
        self.check_is_locked()?;
        let available = if is_chargeback {
            new_total
        } else {
            new_total - self.held
        };
        if available >= Decimal::new(0, 1).round_dp(4) {
            Ok(Self {
                held: self.held,
                total: (self.total - amount).round_dp(4),
                is_locked: self.is_locked,
            })
        } else {
            ApplicationError::err("Account does not have sufficient funds")
        }
    }

    pub fn deposit(self, amount: Decimal) -> ApplicationResult<InternalAccount> {
        self.check_is_locked()?;
        Ok(Self {
            held: self.held,
            total: (self.total + amount).round_dp(4),
            is_locked: self.is_locked,
        })
    }

    fn hold(self, amount: Decimal) -> ApplicationResult<InternalAccount> {
        self.check_is_locked()?;
        if self.held + amount <= self.total {
            Ok(Self {
                held: (self.held + amount).round_dp(4),
                total: self.total,
                is_locked: self.is_locked,
            })
        } else {
            ApplicationError::err("Cannot dispute transaction due to insufficient funds in account")
        }
    }

    fn release_hold(self, amount: Decimal) -> ApplicationResult<InternalAccount> {
        self.check_is_locked()?;
        Ok(Self {
            held: (self.held - amount).round_dp(4),
            total: self.total,
            is_locked: self.is_locked,
        })
    }

    fn chargeback(self, amount: Decimal) -> ApplicationResult<InternalAccount> {
        let mut acc = self.withdraw(amount, true)?;
        acc.held = (acc.held - amount).round_dp(4);
        acc.is_locked = true;
        Ok(acc)
    }
}

fn process_deposit(
    account_store: &mut AccountStore,
    account_opt: Option<InternalAccount>,
    tx: Transaction,
    amount: Decimal,
) -> ApplicationResult<()> {
    let res = match account_opt {
        Some(account) => {
            let new_account = account.deposit(amount)?;
            account_store.accounts.insert(tx.client, new_account);
            Ok(())
        }
        None => {
            account_store
                .accounts
                .insert(tx.client, InternalAccount::new(amount));
            Ok(())
        }
    };

    match res {
        Ok(_) => account_store.add_tx_to_history(tx, amount),
        Err(_) => (),
    }
    res
}

fn process_withdrawal(
    account_store: &mut AccountStore,
    account_opt: Option<InternalAccount>,
    tx: Transaction,
    amount: Decimal,
) -> ApplicationResult<()> {
    match account_opt {
        Some(account) => {
            match account.withdraw(amount, false) {
                Ok(new_account) => {
                    account_store.accounts.insert(tx.client, new_account);
                }
                Err(_) => (),
            }
            Ok(())
        }
        None => {
            account_store
                .accounts
                .insert(tx.client, InternalAccount::new_with_no_funds());
            Ok(())
        }
    }
}

fn process_dispute(
    account_store: &mut AccountStore,
    account: InternalAccount,
    tx: Transaction,
) -> ApplicationResult<()> {
    match account_store.get_historical_tx(tx.tx) {
        Some(historical_tx) => {
            if historical_tx.state == Success {
                let new_account = account.hold(historical_tx.amount)?;
                account_store.accounts.insert(tx.client, new_account);
                account_store.update_tx_state(tx.tx, Disputed);
                Ok(())
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

fn process_chargeback(
    account_store: &mut AccountStore,
    account: InternalAccount,
    tx: Transaction,
) -> ApplicationResult<()> {
    match account_store.get_historical_tx(tx.tx) {
        Some(historical_tx) => {
            if historical_tx.state == Disputed {
                let new_account = account.chargeback(historical_tx.amount)?;
                account_store.accounts.insert(tx.client, new_account);
                account_store.update_tx_state(tx.tx, ChargedBack);
                Ok(())
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

fn process_resolve(
    account_store: &mut AccountStore,
    account: InternalAccount,
    tx: Transaction,
) -> ApplicationResult<()> {
    match account_store.get_historical_tx(tx.tx) {
        Some(historical_tx) => {
            if historical_tx.state == Disputed {
                let new_account = account.release_hold(historical_tx.amount)?;
                account_store.accounts.insert(tx.client, new_account);
                account_store.update_tx_state(tx.tx, Success);
                Ok(())
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

pub struct AccountStore {
    accounts: HashMap<u16, InternalAccount>,
    tx_history: HashMap<u32, HistoricalTransaction>,
}

impl AccountStore {
    fn write_output(&self, writer: &mut Writer<Stdout>) -> ApplicationResult<()> {
        self.accounts.iter().for_each(|(id, internal_account)| {
            let account = internal_account.to_account(id.clone());
            let _ = writer.serialize(account);
        });
        writer.flush()?;
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            tx_history: HashMap::new(),
        }
    }

    fn get_historical_tx(&self, id: u32) -> Option<&HistoricalTransaction> {
        self.tx_history.get(&id)
    }

    fn update_tx_state(&mut self, id: u32, state: TransactionState) {
        let tx_opt = self.tx_history.get(&id);
        match tx_opt {
            Some(tx) => {
                self.tx_history.insert(id, tx.update_state(state));
            }
            None => (),
        }
    }
    fn add_tx_to_history(&mut self, tx: Transaction, amount: Decimal) {
        self.tx_history
            .insert(tx.tx, tx.to_historical_transaction(amount));
    }
    pub fn add_tx(&mut self, tx: Transaction) -> ApplicationResult<()> {
        let account_opt_ref = self.accounts.get(&tx.client);

        match (account_opt_ref.cloned(), tx.tr_type.clone(), tx.amount) {
            (account_opt, TransactionType::Deposit, Some(amount)) => {
                process_deposit(self, account_opt, tx, amount)
            }
            (Some(account), TransactionType::Dispute, ..) => process_dispute(self, account, tx),
            (Some(account), TransactionType::Chargeback, ..) => {
                process_chargeback(self, account, tx)
            }
            (Some(account), TransactionType::Resolve, ..) => process_resolve(self, account, tx),
            (account_opt, TransactionType::Withdrawal, Some(amount)) => {
                process_withdrawal(self, account_opt, tx, amount)
            }
            _ => Ok(()),
        }
    }

    #[cfg(test)]
    pub fn export_accounts(&self) -> Vec<Account> {
        self.accounts
            .iter()
            .map(|(client_id, internal_account)| internal_account.to_account(client_id.clone()))
            .collect()
    }
}
