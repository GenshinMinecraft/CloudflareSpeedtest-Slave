mod return_default;
mod ping;
mod speed;
mod cfst_rpc;

use uuid::Uuid;
use cloudflare_speedtest_client::CloudflareSpeedtestClient;
use log::{error, info, warn};
use return_default::*;
use ping::*;
use simple_logger::init_with_level;
use speed::speed_one_ip;
use cfst_rpc::*;

use clap::{error, Parser};
use tonic::transport::Channel;
use std::{process::exit, sync::Mutex};

/// Cloudflare IP Speedtest Backend
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    /// Frontend Server Address
    #[arg(short, long, default_value_t = return_default_server())]
    server: String,

    /// Token Setting
    #[arg(short, long, default_value_t = return_default_bootstrap_token())]
    token: String,

    /// Bandwidth (in Mbps)
    #[arg(short, long, default_value_t = 500)]
    max_mbps: i32,
}

fn init_args() -> Args {
    let args: Args = Args::parse();
    return args;
}

async fn init_client(server_url: String) -> CloudflareSpeedtestClient<Channel> {

    let mut client = match CloudflareSpeedtestClient::connect("http://".to_string() + &server_url).await {
        Ok(tmp) => {
            info!("成功连接服务器");
            tmp
        },
        Err(e) => {
            error!("无法连接服务器: {}", e);
            exit(1);
        },
    };
    return client;
}

async fn send_bootstrap(client: CloudflareSpeedtestClient<Channel>, maximum_mbps: i32, bootstrap_token: String) -> (BootstrapResponse, String, String) {
    
    let node_id: String = Uuid::new_v4().to_string();
    
    let reqwest: BootstrapRequest = BootstrapRequest {
        maximum_mbps: maximum_mbps,
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        bootstrap_token: bootstrap_token,
        node_id: node_id.clone(),
    };

    let response: BootstrapResponse = match client.clone().bootstrap(reqwest).await {
        Ok(res) => res.get_ref().clone(),
        Err(e) => {
            error!("发送 Bootstrap 时发送错误: {}", e);
            exit(1);
        },
    };

    let session_token: String = response.clone().session_token;

    if response.clone().success != true {
        error!("Bootstrap 信息已成功，但返回错误 (也许是 Bootstrap Token 设置错误): {:?}", response.clone());
        exit(1);
    }

    if response.clone().should_upgrade == true {
        warn!("该从端需更新，建议更新至最新版本");
    }

    return (response, node_id, session_token);
}

#[tokio::main]
async fn main() {
    init_with_level(log::Level::Debug).unwrap();
    let args: Args = init_args();
    let client: CloudflareSpeedtestClient<Channel> = init_client(args.server).await;

    let (_, node_id, session_token) = send_bootstrap(client, args.max_mbps, args.token).await;

    print!("{}, {}", node_id, session_token);
    
    return; 
}