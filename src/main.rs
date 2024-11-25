mod engine;
mod models;
mod tests;

use crate::engine::process_file;
use crate::models::ApplicationResult;

fn main() -> ApplicationResult<()> {
    process_file()
}
