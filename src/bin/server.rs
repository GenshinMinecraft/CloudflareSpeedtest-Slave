use std::net::SocketAddr;

use volo_grpc::server::{Server, ServiceBuilder};

use cloudflare_speedtest_slave::S;

#[volo::main]
async fn main() {
    let addr: SocketAddr = "[::]:11451".parse().unwrap();
    let addr = volo::net::Address::from(addr);

    Server::new()
        .add_service(
            ServiceBuilder::new(volo_gen::cfst_rpc::CloudflareSpeedtestServer::new(S)).build(),
        )
        .run(addr)
        .await
        .unwrap();
}
