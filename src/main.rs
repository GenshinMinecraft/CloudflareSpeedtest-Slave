mod ping;

use cfst_rpc::{BootstrapRequest, BootstrapResponse, Ping, Pong, SpeedtestRequest, SpeedtestResponse, UpgradeRequest, UpgradeResponse };
use cfst_rpc::cloudflare_speedtest_server::{CloudflareSpeedtest, CloudflareSpeedtestServer};
use tonic::async_trait;
use tonic::{transport::Server, Request, Response, Status};
use crate::ping::*;
use log::{Level, info};
use simple_logger;

pub mod cfst_rpc {
    tonic::include_proto!("cfst_rpc");
}

#[derive(Default)]
pub struct MyCloudflareSpeedtest {}

#[async_trait]
impl CloudflareSpeedtest for MyCloudflareSpeedtest {
    // 实现 Bootstrap 方法
    async fn bootstrap(&self, request: Request<BootstrapRequest>) -> Result<Response<BootstrapResponse>, Status> {
        info!("Got BootstrapRequest: {:?}", request);
        let response = BootstrapResponse { success: todo!(), should_upgrade: todo!(), message: todo!(), session_token: todo!() };
        Ok(Response::new(response))
    }

    // 实现 Upgrade 方法
    async fn upgrade(&self, request: Request<UpgradeRequest>) -> Result<Response<UpgradeResponse>, Status> {
        info!("Got UpgradeRequest: {:?}", request);
        let response = UpgradeResponse { success: todo!(), message: todo!(), upgrade_url: todo!() };
        Ok(Response::new(response))
    }

    // 实现 Speedtest 方法
    async fn speedtest(&self, request: Request<SpeedtestRequest>) -> Result<Response<SpeedtestResponse>, Status> {
        info!("Got SpeedtestRequest: {:?}", request);
        let response = SpeedtestResponse { success: todo!(), message: todo!(), province: todo!(), isp: todo!(), ip_results: todo!(), session_token: todo!(), node_id: todo!() };
        Ok(Response::new(response))
    }

    // 实现 Alive 方法
    async fn alive(&self, request: Request<Ping>) -> Result<Response<Pong>, Status> {
        info!("Got Ping: {:?}", request);
        let response = Pong {};
        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 Logger
    simple_logger::init_with_level(Level::Debug).unwrap();

    let addr = "[::1]:50051".parse()?; // 监听地址
    let service = MyCloudflareSpeedtest::default();


    Server::builder()
        .add_service(CloudflareSpeedtestServer::new(service))
        .serve(addr)
        .await;

    Ok(())
}