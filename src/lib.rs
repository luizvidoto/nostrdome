pub mod app;
pub(crate) mod components;
mod config;
pub(crate) mod consts;
pub mod db;
pub(crate) mod error;
pub(crate) mod icon;
pub mod net;
pub(crate) mod style;
pub mod types;
pub mod utils;
pub(crate) mod views;
pub(crate) mod widget;
pub(crate) use crate::error::Error;

use tracing_subscriber::{
    fmt::SubscriberBuilder, prelude::__tracing_subscriber_SubscriberExt, EnvFilter,
};

pub fn setup_logger() {
    // Cria um filtro de ambiente que define o nível de log padrão para todas as bibliotecas como ERROR e o nível de log do seu aplicativo como INFO
    let filter = EnvFilter::from_default_env()
        .add_directive("nostrtalk=info".parse().unwrap())
        .add_directive("warn".parse().unwrap());

    let subscriber = SubscriberBuilder::default()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        // .with_writer(non_blocking)
        .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::default()) // Adicione esta linha para incluir eventos de spans
        .finish()
        .with(tracing_error::ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    tracing::info!("Starting up");
}
