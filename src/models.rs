use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io::Error;

#[derive(Debug)]
pub struct ApplicationError {
    pub msg: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Chargeback,
    Deposit,
    Dispute,
    Resolve,
    Withdrawal,
}

impl From<csv::Error> for ApplicationError {
    fn from(value: csv::Error) -> Self {
        ApplicationError::new(value.to_string().as_str())
    }
}

impl From<Error> for ApplicationError {
    fn from(value: Error) -> Self {
        ApplicationError::new(value.to_string().as_str())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub tr_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

impl Transaction {

    pub fn new(tr_type: TransactionType, client: u16, tx: u32, amount: Option<Decimal>) -> Self {
        Self {
            tr_type,
            client,
            tx,
            amount
        }
    }
    pub fn to_historical_transaction(&self, amount: Decimal) -> HistoricalTransaction {
        HistoricalTransaction {
            tr_type: self.tr_type.clone(),
            client: self.client,
            tx: self.tx,
            amount,
            state: TransactionState::Success,
        }
    }
}

#[derive(PartialEq)]
pub enum TransactionState {
    ChargedBack,
    Disputed,
    Success,
}

pub struct HistoricalTransaction {
    pub tr_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Decimal,
    pub state: TransactionState,
}

impl HistoricalTransaction {
    pub fn update_state(&self, state: TransactionState) -> HistoricalTransaction {
        Self {
            tr_type: self.tr_type.clone(),
            client: self.client,
            tx: self.tx,
            amount: self.amount,
            state,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Account {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn new(client: u16, available: Decimal, held: Decimal, total: Decimal, locked: bool) -> Self {
        Self {
            client,
            available,
            held,
            total,
            locked
        }
    }
}

impl ApplicationError {
    pub fn new(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
        }
    }

    pub fn err<A>(msg: &str) -> ApplicationResult<A> {
        Err(ApplicationError::new(msg))
    }
}

pub type ApplicationResult<A> = Result<A, ApplicationError>;
