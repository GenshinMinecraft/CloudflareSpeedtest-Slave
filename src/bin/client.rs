use std::{
    net::SocketAddr, process::exit, sync::Mutex
};

#[path = "../ping.rs"]
mod ping;
#[path = "../speed.rs"]
mod speed;
#[path = "../return_default.rs"]
mod return_default;

use crate::ping::ping_ips;
use crate::speed::speed_one_ip;
use crate::return_default::*;

use ping::ip_cidr_to_ips;
use tokio_stream::StreamExt;
use uuid::Uuid;
use volo_grpc::Status;
use volo_gen::cfst_rpc::*;
use faststr::FastStr;
use log::{*, Level::*};
use simple_logger::*;
use clap::Parser;
use lazy_static::lazy_static;
use std::collections::HashMap;

static ARGS: Mutex<Args> = Mutex::new(Args { server: String::new(), token: String::new(), max_mbps: 500 });
static SERVER_URL: Mutex<String> = Mutex::new(String::new());
static SESSION_TOKEN: Mutex<String> = Mutex::new(String::new());
static NODE_ID: Mutex<String> = Mutex::new(String::new());

lazy_static! {
    static ref CLIENT: CloudflareSpeedtestClient = {
        let server = SERVER_URL.lock().unwrap();
        
        let server_url = server.clone();

        drop(server);
        
        let addr: SocketAddr = server_url.parse().unwrap();
        volo_gen::cfst_rpc::CloudflareSpeedtestClientBuilder::new("cfst_rpc")
            .address(addr)
            .build()
    };
}



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

fn init_args() {
    let args: Args = Args::parse();

    let mut tmp_args = ARGS.lock().unwrap();
    *tmp_args = args.clone();
    drop(tmp_args);
    drop(args);
}

async fn init_client(server_url: String) {
    let mut tmpserver = SERVER_URL.lock().unwrap();
    *tmpserver = server_url;

    drop(tmpserver);

    let _ = &CLIENT;
    return;
}

async fn send_bootstrap() -> BootstrapResponse {
    let reqwest = BootstrapRequest {
        maximum_mbps: ARGS.lock().unwrap().max_mbps.clone(),
        client_version: FastStr::from_static_str(env!("CARGO_PKG_VERSION")),
        bootstrap_token: FastStr::from_string(ARGS.lock().unwrap().token.clone()),
        node_id: FastStr::from_string(Uuid::new_v4().to_string()),
    };

    let mut tmp_node_id = NODE_ID.lock().unwrap();
    *tmp_node_id = reqwest.node_id.to_string();
    drop(tmp_node_id);

    let mut response: BootstrapResponse = BootstrapResponse { success: false, should_upgrade: false, message: todo!(), session_token: todo!() };
    match CLIENT.bootstrap(reqwest).await {
        Ok(res) => response = res.get_ref().clone(),
        Err(e) => {
            error!("{}", e);
            exit(1);
        },
    }

    return response;
}

fn set_session_token(bootstrap_req: BootstrapResponse) {
    let mut tmp_session_token = SESSION_TOKEN.lock().unwrap();
    *tmp_session_token = bootstrap_req.session_token.to_string();
    drop(tmp_session_token);
} 

async fn send_speedtest() -> Result<SpeedtestResponse, Status>{
    let request = SpeedtestRequest {
        session_token: FastStr::from_string(SESSION_TOKEN.lock().unwrap().to_string()),
        node_id: FastStr::from_string(NODE_ID.lock().unwrap().to_string()),
    };

    let mut response = match CLIENT.speedtest(request).await {
        Ok(resp) => resp.into_inner(),
        Err(e) => {
            error!("Can not get gRPC stream from server: {:?}", e);
            return Err(e);
        }
    };

    let mut speedtest_response: SpeedtestResponse = SpeedtestResponse { ip_ranges: Vec::new(), minimum_mbps: -1, maximum_ping: -1, speed_url: FastStr::new("") };

    loop {
        match response.next().await {
            Some(Ok(info)) => {
                speedtest_response = info;
                break;
            }
            Some(Err(e)) => {
                error!("Can not get gRPC stream from server: {:?}", e);
                return Err(e);
            }
            None => {
                break;
            }
        }
    }

    Ok(speedtest_response)
}



#[volo::main]
async fn main() {
    // Init Part
    init_with_level(Info).unwrap();
    init_args();
    init_client(ARGS.lock().unwrap().server.clone()).await;
    info!("INITED Cloudflare IP Speedtest Backend");

    // 发送 Bootstrap
    let bootstrap_req = send_bootstrap().await;
    set_session_token(bootstrap_req.clone());
    drop(bootstrap_req);

    info!("Got Bootstrap Response");

    // 发送 Speedtest
    loop {

        info!("Send Speedtest Request");

        let process_message = match send_speedtest().await {
            Ok(tmp) => tmp,
            Err(e) => {
                warn!("Can not get speedtest IP: {}, Retrying", e);
                break;
            },
        };

        info!("Got Speedtest Request");

        let ip_ranges = process_message.ip_ranges;
        
        let ips = match ip_cidr_to_ips(ip_ranges).await {
            Ok(tmp) => tmp,
            Err(e) => {
                error!("Can not parse the ip CIDR: {}", e);
                continue;
            },
        };

        info!("Ping IPs");

        let ips_rtt = ping_ips(ips.clone()).await;
        
        let mut ips_rtt_map: HashMap<String, f64> = HashMap::new();
        for (key, value) in ips.into_iter().zip(ips_rtt.into_iter()) {
            ips_rtt_map.insert(key, value);
        }

        info!("Speedtesting");

        let mut speed_ips: Vec<String> = Vec::new();
        for (ip, ping_rtt) in ips_rtt_map.clone() {
            if (ping_rtt * 1000.0).round() as i32 > process_message.maximum_ping {
                speed_ips.push(ip);
            } else {
                continue;
            }
        }

//        let random_speed_ips: Vec<String> = speed_ips.choose_multiple(&mut thread_rng(), 10).cloned().collect();

        let mut the_last_ip = String::new();
        let mut the_last_ip_speed: i32 = 0;
        for speed_ip in speed_ips {
            let ip_bandwidth = speed_one_ip(process_message.speed_url.to_string(), speed_ip.clone(), 5).await;
            if ip_bandwidth >= process_message.minimum_mbps as f64 {
                the_last_ip = speed_ip;
                the_last_ip_speed = ip_bandwidth as i32;
                break;
            }            
        }
        let the_last_ip_rtt_borrow: &f64 = ips_rtt_map.get(&the_last_ip).unwrap();
        let the_last_ip_rtt_sec: f64 = *the_last_ip_rtt_borrow;
        let the_last_ip_rtt_ms: f64 = the_last_ip_rtt_sec * 1000.0;
        let the_last_ip_rtt = the_last_ip_rtt_ms.round() as i32;

        let mut ipresult_vec: Vec<IpResult> = Vec::new();

        ipresult_vec.push(IpResult {
            ip_address: FastStr::new(the_last_ip), 
            latency: the_last_ip_rtt, 
            speed: the_last_ip_speed,
        });
        let speedtest_result_request: SpeedtestResultRequest = SpeedtestResultRequest { 
            ip_results: ipresult_vec, 
            session_token: FastStr::from_string(SESSION_TOKEN.lock().unwrap().to_string()), 
            node_id: FastStr::from_string(NODE_ID.lock().unwrap().to_string()),
        };

        CLIENT.speedtest_result(speedtest_result_request);
    }
}


