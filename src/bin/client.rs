use std::{
    io::{Read, Write},
    net::{TcpStream, SocketAddr},
    process::exit,
    sync::Mutex,
    time::Instant,
};

use native_tls::TlsConnector;
use tokio_stream::StreamExt;
use url::Url;
use uuid::Uuid;
use volo_grpc::Status;
use volo_gen::cfst_rpc::*;
use faststr::FastStr;
use log::{*, Level::*};
use simple_logger::*;
use clap::Parser;
use lazy_static::lazy_static;
use std::time::Duration;
use fastping_rs::PingResult::{Idle, Receive};
use fastping_rs::Pinger;


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

fn return_default_server() -> String {
    return "1.1.1.1:1145".to_string();
}

fn return_default_bootstrap_token() -> String {
    return "admin".to_string();
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

async fn speed_one_ip(speedtest_url: String, ip: String, speed_time: u32) -> i128 {

    let url = match Url::parse(speedtest_url.as_str()) {
        Ok(parsed_url) => parsed_url,
        Err(_) => {
            eprintln!("Failed to parse URL.");
            return -1;
        },
    };

    let domain = url.domain().unwrap();
    let port = url.port().unwrap_or(443);
    let path = url.path();

    let stream = TcpStream::connect(format!("{}:{}", ip, port)).unwrap();
    let connector = TlsConnector::builder().build().unwrap();
    let mut stream = connector.connect(domain, stream).unwrap();
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, domain
    );

    let start_time = Instant::now();
    let mut buffer = [0; 10240];
    let mut data = 0;

    stream.write_all(request.as_bytes()).unwrap();

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break, 
            Ok(n) => {
                data += n;
                
                if start_time.elapsed().as_secs_f64() >= speed_time as f64 {
                    break;
                }
            },
            Err(_e) => break,
        }
    }

    let time_seconds = start_time.elapsed().as_secs(); // 下载所花的时间，单位：秒，类型为 u64
    let megabytes_downloaded = data as f64 / 1_048_576.0;
    let time_seconds_f64 = time_seconds as f64;
    let download_speed_megabytes_per_second = megabytes_downloaded / time_seconds_f64;
    let download_speed_megabits_per_second = download_speed_megabytes_per_second * 8.0;

    download_speed_megabits_per_second.round() as i128
}


fn duration_to_f64(duration: Duration) -> f64 {
    // 获取整个秒数
    let seconds = duration.as_secs() as f64;
    let nanos = duration.subsec_nanos() as f64 / 1e9;
    return seconds + nanos;
}

async fn ping_ips(ips: Vec<String>) -> Vec<f64>{
    let (pinger, results) = match Pinger::new(Some(1000), Some(56)) {
        Ok((pinger, results)) => (pinger, results),
        Err(e) => {
            error!("新建 Pinger 时候出错 (不是哥们这都能报错？): {}", e);
            panic!("{}", e)
        },
    };

    for ip in ips.clone() {
        pinger.add_ipaddr(&ip);
    }

    pinger.run_pinger();

    let mut ips_rtt: Vec<f64> = Vec::new();

    loop {
        match results.recv() {
            Ok(result) => match result {
                Idle { addr } => {
                    warn!("无效/不可达的 IP:  {}.", addr);
                    ips_rtt.push(-1.0);
                    
                    if ips_rtt.len() == ips.len() {
                        pinger.stop_pinger();
                        break;
                    }
                }
                Receive { addr, rtt } => {
                    info!("存活 IP: {} in {:?}.", addr, rtt);
                    ips_rtt.push(duration_to_f64(rtt));

                    if ips_rtt.len() == ips.len() {
                        pinger.stop_pinger();
                        break;
                    }
                }
            },
            Err(e) => {
                error!("获取 IP 测试结果时出现错误: {}", e);    
            },
        }
    }

    return ips_rtt;
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
        let process_message = match send_speedtest().await {
            Ok(tmp) => tmp,
            Err(e) => {
                warn!("Can not get speedtest IP: {}, Retrying", e);
                break;
            },
        };

        let ip_list = process_message.ip_ranges;
    }
}


