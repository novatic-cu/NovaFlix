//! Ejecuta el diagnóstico de red sin interfaz e imprime el informe (sin contraseñas).
//! `cargo run -p novaflix-core --example probe`

use novaflix_core::{transport::probe, Net, NetConfig};

#[tokio::main]
async fn main() {
    let net = Net::build(NetConfig::default()).expect("red");
    let started = std::time::Instant::now();
    let report = probe::run(&net).await;
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    eprintln!("total: {:?}", started.elapsed());
}
