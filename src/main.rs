mod return_default;
mod ping;
mod speed;
mod cfst_rpc;

use tokio_stream::StreamExt;
use uuid::Uuid;
use cloudflare_speedtest_client::CloudflareSpeedtestClient;
use log::{error, info, warn};
use return_default::*;
use ping::*;
use simple_logger::init_with_level;
use speed::speed_one_ip;
use cfst_rpc::*;

use clap::Parser;
use tonic::transport::Channel;
use std::{env, error::Error, fs::{self, File}, io::{self, Write}, process::{exit, Command}};

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

    /// Enable Debug Log
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Install For Systemd
    #[arg(long, default_value_t = false)]
    install: bool,
}

fn init_args() -> Args {
    let args: Args = Args::parse();
    return args;
}

fn install_systemd(args: Args) {
    if env::consts::OS != "linux" {
        error!("Install 功能仅适用于 Linux 系统");
        exit(1);
    }

    match fs::metadata("/usr/bin/systemctl") {
        Ok(_) => {
            info!("您的系统使用的是 Systemd 服务管理器, 可以正常使用 Install 功能")
        },
        Err(_) => {
            error!("您的系统并非使用 Systemd 作为服务管理器, 无法使用 Install 功能, 请自行配置进程守护");
            exit(1);
        },
    }

    if std::env::var("USER") == Ok("root".to_string()) {
        info!("正在使用 root 用户");
    } else {
        error!("非 root 用户, 请使用 root 用户运行 Install 功能");
        exit(1);
    }

    match fs::metadata("/etc/systemd/system/cfst_slave.service") {
        Ok(_) => {
            loop {
                warn!("您的当前系统曾经配置过 Systemd 保活服务, 是否覆盖? (Y/N)");
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                input = input.trim().to_uppercase();
        
                if input == "Y" {
                    info!("正在为您覆盖先前的文件");
                    break;
                } else if input == "N" {
                    info!("不覆盖, 退出程序");
                    exit(1);
                } else {
                    warn!("输入错误，请重新输入 Y 或 N。");
                }
            }
        },
        Err(_) => {},
    }

    match env::current_exe() {
        Ok(path_to_bin) => {
            if path_to_bin.to_str().unwrap() == "/usr/bin/CloudflareSpeedtest-Slave" {
                info!("无需复制可执行文件");
            } else {
                match Command::new("cp").arg("-afr").arg(path_to_bin).arg("/usr/bin/CloudflareSpeedtest-Slave").output() {
                    Ok(tmp) => {
                        if tmp.status.success() {
                            info!("成功将可执行文件复制到 /usr/bin/CloudflareSpeedtest-Slave");
                        } else {
                            error!("无法将可执行文件复制到 /usr/bin/CloudflareSpeedtest-Slave");
                            exit(1);
                        }
                    },
                    Err(e) => {
                        error!("无法将可执行文件复制到 /usr/bin/CloudflareSpeedtest-Slave: {}", e);
                        exit(1);
                    },
                }
            }
        },
        Err(e) => {
            error!("无法获取可执行文件路径: {}", e);
            exit(1);
        }
    }    

    let service_config = r#"[Unit]
Description=Cloudflare Speedtest Slave
After=network.target

[Install]
WantedBy=multi-user.target

[Service]
Type=simple
ExecStart=/usr/bin/CloudflareSpeedtest-Slave -s SERVERURL -t TOKEN -m MAXMBPS
Restart=always
"#;

    let mut replaced_service_config = service_config.replace("SERVERURL", args.server.as_str());
    replaced_service_config = replaced_service_config.replace("TOKEN", args.token.as_str());
    if args.debug {
        let tmp = args.max_mbps.to_string() + " --debug";
        replaced_service_config = replaced_service_config.replace("MAXMBPS", tmp.as_str());
    } else {
        replaced_service_config = replaced_service_config.replace("MAXMBPS", args.max_mbps.to_string().as_str());
    }

    match Command::new("rm").arg("-rf").arg("/etc/systemd/system/cfst_slave.service").output() {
        Ok(_) => {},
        Err(e) => {
            error!("无法删除 /etc/systemd/system/cfst_slave.service: {}", e);
            exit(1);
        },
    }

    let mut service_file = match File::create("/etc/systemd/system/cfst_slave.service") {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法新建 /etc/systemd/system/cfst_slave.service: {}", e);
            exit(1);
        },
    };

    match service_file.write_all(replaced_service_config.as_bytes()) {
        Ok(_) => {
            info!("成功写入 Systemd 配置文件")
        },
        Err(e) => {
            error!("无法写入 Systemd 配置文件: {}", e);
            exit(1);
        },
    }
    
    match Command::new("systemctl").arg("daemon-reload").output() {
        Ok(tmp) => {
            if tmp.status.success() {
                info!("成功运行 systemctl daemon-reload");
            } else {
                error!("无法运行 systemctl daemon-reload")
            }
        },
        Err(e) => {
            error!("无法运行 systemctl daemon-reload: {}", e);
            exit(1);
        },
    }

    match Command::new("systemctl").arg("start").arg("cfst_slave.service").output() {
        Ok(tmp) => {
            if tmp.status.success() {
                info!("成功启动 Cloudflare Speedtest Slave");
            } else {
                error!("无法启动 Cloudflare Speedtest Slave")
            }
        },
        Err(e) => {
            error!("无法启动 Cloudflare Speedtest Slave: {}", e);
            exit(1);
        },
    }

    loop {
        info!("是否打开开机自启? (Y/N)");
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input = input.trim().to_uppercase();

        if input == "Y" {
            match Command::new("systemctl").arg("enable").arg("cfst_slave.service").output() {
                Ok(tmp) => {
                    if tmp.status.success() {
                        info!("成功打开开机自启");
                    } else {
                        error!("无法打开开机自启")
                    }
                },
                Err(e) => {
                    error!("无法打开开机自启: {}", e);
                    exit(1);
                },
            }
            break;
        } else if input == "N" {
            info!("不打开, 退出程序");
            exit(1);
        } else {
            warn!("输入错误，请重新输入 Y 或 N。");
        }
    }
}

async fn init_client(server_url: String) -> CloudflareSpeedtestClient<Channel> {

    let client = match CloudflareSpeedtestClient::connect("http://".to_string() + &server_url).await {
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
        error!("Bootstrap 信息已成功, 但返回错误 (也许是 Bootstrap Token 设置错误): {:?}", response.clone());
        exit(1);
    }

    if response.clone().should_upgrade == true {
        warn!("该从端需更新, 建议更新至最新版本");
    }

    return (response, node_id, session_token);
}

async fn send_speedtest(client: CloudflareSpeedtestClient<Channel>, node_id: String, session_token: String) -> Result<(SpeedtestResponse, Vec<String>), Box<dyn Error>> {
    let reqwest = SpeedtestRequest {
        session_token,
        node_id,
    };

    let stream = match client.clone().speedtest(reqwest).await {
        Ok(tmp) => {
            tmp.into_inner()
        },
        Err(e) => {
            return Err(Box::new(e));
        },
    };

    let mut stream = stream.take(1);
    let response = match stream.next().await {
        Some(tmp) => tmp?,
        None => return Err("无法获取任何 Speedtest 回应".into()),
    };

    let ip_ranges_ips = ip_cidr_to_ips(response.clone().ip_ranges).await?;

    Ok((response, ip_ranges_ips))
}

async fn send_speedtest_result(ip: String, ping: i32, speed: i32, mut client: CloudflareSpeedtestClient<Channel>, node_id: String, session_token: String) -> Result<SpeedtestResultResponse, Box<dyn Error>> {
    let ipresult = IpResult {
        ip_address: ip,
        latency: ping,
        speed,
    };

    let ipresult_vec: Vec<IpResult> = vec![ipresult];

    let reqwest = SpeedtestResultRequest {
        ip_results: ipresult_vec,
        session_token,
        node_id,
    };

    match client.speedtest_result(reqwest).await {
        Ok(tmp) => {
            info!("成功发送 Speedtest Result 信息");
            return Ok(tmp.get_ref().clone());
        },
        Err(e) => {
            error!("无法发送 Speedtest Result 信息: {}", e);
            return Err(Box::new(e));
        },
    }
}

#[tokio::main]
async fn main() {

    if env::consts::OS == "windows" {
        error!("天灭 Windows, Linux/OSX 保平安！");
        error!("由于 fastping-rs 库不支持 Windows, 所以本项目永远不会支持 Windows");
        error!("即使您在 Windows 环境下编译通过, 也绝不可能正常运行！");
        error!("如果您真的需要在 Windows 下运行, 请使用其他项目: 暂无");
        exit(1);
    }

    let args: Args = init_args();
    if args.debug {
        init_with_level(log::Level::Debug).unwrap();
    } else {
        init_with_level(log::Level::Info).unwrap();
    }
    if args.install {
        install_systemd(args);
        exit(1);
    }

    let client: CloudflareSpeedtestClient<Channel> = init_client(args.server).await;

    loop {
        let (_, node_id, session_token) = send_bootstrap(client.clone(), args.max_mbps, args.token.clone()).await;

        info!("当前 Node_ID: {}, Session_token: {}", node_id, session_token);

        let (speedtest_response, need_ping_ips) = match send_speedtest(client.clone(), node_id.clone(), session_token.clone()).await {
            Ok((res, str)) => {
                info!("成功获取 Speedtest 信息, 开始启动测速程序");
                (res, str)
            },
            Err(e) => {
                error!("未能成功获取需要测试的 IP, 正在重试: {}", e);
                continue;
            },
        };

        let speedtest_url = speedtest_response.speed_url;
        let speedtest_minimum_mbps = speedtest_response.minimum_mbps;
        let speedtest_maximum_ping = speedtest_response.maximum_ping;

        let mut ips_ping: std::collections::HashMap<String, u128> = ping_ips(need_ping_ips, speedtest_maximum_ping).await;
        info!("总计 IP 有 {} 个", ips_ping.len());
        ips_ping.retain(|_, &mut value| value != u128::MAX);
        info!("符合条件 IP 有 {} 个", ips_ping.len());

        let mut the_last_ip: String = String::new();
        let mut the_last_ip_ping: i32 = -1;
        let mut the_last_ip_speed: i32 = -1;
        for (speed_ip, _) in ips_ping.clone() {
            let tmp_speed = speed_one_ip(speedtest_url.clone(), speed_ip.clone(), 10).await;
            if tmp_speed.round() as i32 >= speedtest_minimum_mbps {
                the_last_ip = speed_ip;
                the_last_ip_ping = *ips_ping.get(&the_last_ip).unwrap() as i32;
                the_last_ip_speed = tmp_speed.round() as i32;
                break;
            } else {
                continue;
            }
        }

        match send_speedtest_result(the_last_ip, the_last_ip_ping, the_last_ip_speed, client.clone(), node_id.clone(), session_token.clone()).await {
            Ok(_) => info!("成功完成一次 Speedtest, 开始继续接受 Speedtest 信息"),
            Err(_) => todo!(),
        }

    }
}