use lazy_static::lazy_static;
use volo_gen::cfst_rpc::*;
use faststr::FastStr;
use volo_http::Address;
use std::net::SocketAddr;
use simple_logger::*;
use log::Level::*;
use clap::Parser;
use std::sync::Mutex;

lazy_static! {
    static ref CLIENT: CloudflareSpeedtestClient = {
        let addr: SocketAddr = "[::1]:8080".parse().unwrap();
        volo_gen::cfst_rpc::CloudflareSpeedtestClientBuilder::new("volo-example")
            .address(addr)
            .build()
    };
}

fn return_default_server() -> String {
    return "1.1.1.1:1145".to_string();
}

fn return_default_bootstrap_token() -> String {
    return "admin".to_string();
}

/// Cloudflare IP Speedtest Backend
#[derive(Parser, Debug)]
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

    /// Backend Node ID
    #[arg(short, long)]
    node_id: i32, 
}

#[volo::main]
async fn main() {
    // Init Log
   init_with_level(Info).unwrap();

   let args: Args = Args::parse(); 
}
