use std::{net::SocketAddr, process::exit};
use volo_grpc::server::{Server, ServiceBuilder};
use cloudflare_speedtest_slave::S;
use log::{Level, info, warn, error, debug};
use simple_logger;

#[volo::main]
async fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    info!("正在启动 CloudflareSpeedtest-Slave");

    let addr: SocketAddr = "[::]:11451".parse().unwrap();
    let addr: volo::net::Address = volo::net::Address::from(addr);

    Server::new().add_service(ServiceBuilder::new(volo_gen::cfst_rpc::CloudflareSpeedtestServer::new(S)).build()).run(addr).await.unwrap()
}
