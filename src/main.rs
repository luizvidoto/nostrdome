#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dotenv::dotenv;
use nostrtalk::app;
use nostrtalk::setup_logger;

#[tokio::main]
async fn main() {
    dotenv().ok();

    setup_logger();

    app::run().await;
}
