use std::{net::SocketAddr, process::exit};
use volo::net::tls::ServerTlsConfig;
use volo_grpc::server::{Server, ServiceBuilder};
use cloudflare_speedtest_slave::S;
use log::{Level, info, warn, error, debug};
use simple_logger;

#[volo::main]
async fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    info!("正在启动 CloudflareSpeedtest-Slave");

    let data_dir = std::path::PathBuf::from_iter([std::env!("CARGO_MANIFEST_DIR"), "tls_cert"]);
    let tls_config = ServerTlsConfig::from_pem_file(
        data_dir.join("server.pem"),
        data_dir.join("server.key"),
    )
    .expect("failed to load certs");

    let addr: SocketAddr = "[::]:11451".parse().unwrap();
    let addr: volo::net::Address = volo::net::Address::from(addr);

    Server::new()
        .tls_config(tls_config)
        .add_service(ServiceBuilder::new(volo_gen::cfst_rpc::CloudflareSpeedtestServer::new(S)).build())
        .run(addr)
        .await
        .unwrap() 
}
