mod engine;
mod models;
#[cfg(test)]
mod tests;

use crate::engine::process_file;
use crate::models::ApplicationResult;

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    process_file().await
}
