# Transactions Demo

This explores modeling deposit and withdrawal transactions in Rust while handling disputes, chargebacks, and resolutions to deposits. It intentionally does not handle disputes to withdrawals.

## Getting Started

Run `cargo run -- transactions.csv > accounts.csv` to see all transactions handled and accounts balanced, with the final balanced outputted to a `accounts.csv`.

Run `cargo test` to run all unit tests.

## Notes

1. Failed transactions (such as withdrawals or disputes where there are not enough funds to withdraw or dispute) fail silently. Future improvements could include collecting these in an error store for automatic retries at a later point or for output to the user. This implementation captures all errors in the `ApplicationError` struct, which would facilitate a future feature to export these.
2. 