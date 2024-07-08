mod return_default;
mod ping;
mod speed;
mod cfst_rpc;

use reqwest::Client;
use tokio_stream::StreamExt;
use uuid::Uuid;
use cloudflare_speedtest_client::CloudflareSpeedtestClient;
use log::{debug, error, info, warn};
use return_default::*;
use ping::*;
use simple_logger::init_with_level;
use speed::speed_one_ip;
use cfst_rpc::*;
use clap::Parser;
use tonic::transport::Channel;
use std::{env, error::Error, fs::{self, File}, io::{self, Write}, process::{exit, Command}};

// 定义参数结构体
/// Cloudflare IP Speedtest Backend
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    // 主端地址
    /// Frontend Server Address
    #[arg(short, long, default_value_t = return_default_server())]
    server: String,

    // Bootstrap Token 设置
    /// Token Setting
    #[arg(short, long, default_value_t = return_default_bootstrap_token())]
    token: String,

    // 最大带宽
    /// Bandwidth (in Mbps)
    #[arg(short, long, default_value_t = 500)]
    max_mbps: i32,

    // Debug Log 设置
    /// Enable Debug Log
    #[arg(long, default_value_t = false)]
    debug: bool,

    // 开始 Install
    /// Install For Systemd
    #[arg(long, default_value_t = false)]
    install: bool,

    // 关闭自动更新
    /// Disable Auto Upgrade ModeD
    #[arg(long, default_value_t = false)]
    disable_auto_upgrade: bool,
}

// 参数初始化
fn init_args() -> Args {
    let args: Args = Args::parse();
    return args;
}

// Install 安装
fn install_systemd(args: Args) {
    // 仅适用于 Linux
    if env::consts::OS != "linux" {
        error!("Install 功能仅适用于 Linux 系统");
        exit(1);
    }
    
    // 检测是否使用 Systemd
    match fs::metadata("/usr/bin/systemctl") {
        Ok(_) => {
            info!("您的系统使用的是 Systemd 服务管理器, 可以正常使用 Install 功能")
        },
        Err(_) => {
            error!("您的系统并非使用 Systemd 作为服务管理器, 无法使用 Install 功能, 请自行配置进程守护");
            exit(1);
        },
    }

    // 判断是否为 Root
    if std::env::var("USER") == Ok("root".to_string()) {
        info!("正在使用 root 用户");
    } else {
        error!("非 root 用户, 请使用 root 用户运行 Install 功能");
        exit(1);
    }

    // 判断是否已经安装过
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
                    warn!("输入错误, 请重新输入 Y 或 N。");
                }
            }
        },
        Err(_) => {},
    }

    // 复制可执行文件
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

    // 设置 service 文件模板
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

    // 修改 service 模板
    let mut replaced_service_config = service_config.replace("SERVERURL", args.server.as_str());
    replaced_service_config = replaced_service_config.replace("TOKEN", args.token.as_str());
    if args.debug {
        let tmp = args.max_mbps.to_string() + " --debug";
        replaced_service_config = replaced_service_config.replace("MAXMBPS", tmp.as_str());
    } else {
        replaced_service_config = replaced_service_config.replace("MAXMBPS", args.max_mbps.to_string().as_str());
    }

    // 删除原先的 service 文件
    match Command::new("rm").arg("-rf").arg("/etc/systemd/system/cfst_slave.service").output() {
        Ok(_) => {},
        Err(e) => {
            error!("无法删除 /etc/systemd/system/cfst_slave.service: {}", e);
            exit(1);
        },
    }

    // 新建文件
    let mut service_file = match File::create("/etc/systemd/system/cfst_slave.service") {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法新建 /etc/systemd/system/cfst_slave.service: {}", e);
            exit(1);
        },
    };

    // 写入文件
    match service_file.write_all(replaced_service_config.as_bytes()) {
        Ok(_) => {
            info!("成功写入 Systemd 配置文件")
        },
        Err(e) => {
            error!("无法写入 Systemd 配置文件: {}", e);
            exit(1);
        },
    }
    
    // systemctl daemon-reload
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

    // 开启服务
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

    // 开机自启
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
            warn!("输入错误, 请重新输入 Y 或 N。");
        }
    }
}

// 初始化 Grpc Client
async fn init_client(server_url: String) -> CloudflareSpeedtestClient<Channel> {
    // 构建 URL
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

// 发送 Bootstrap 请求
async fn send_bootstrap(client: CloudflareSpeedtestClient<Channel>, maximum_mbps: i32, bootstrap_token: String) -> (BootstrapResponse, String, String) {
    // 初始化 Node_ID
    let node_id: String = Uuid::new_v4().to_string();
    
    // 初始化请求信息
    let reqwest: BootstrapRequest = BootstrapRequest {
        maximum_mbps: maximum_mbps,
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        bootstrap_token: bootstrap_token,
        node_id: node_id.clone(),
    };

    debug!("BootStrapRequest Message: {:?}", reqwest);

    // 发送请求
    let response: BootstrapResponse = match client.clone().bootstrap(reqwest).await {
        Ok(res) => res.get_ref().clone(),
        Err(e) => {
            error!("发送 Bootstrap 时发送错误: {}", e);
            exit(1);
        },
    };

    debug!("BootStrapResponse Message: {:?}", response);

    // 检测返回 
    if response.clone().success != true {
        error!("Bootstrap 信息已成功, 但返回错误 (也许是 Bootstrap Token 设置错误): {:?}", response.clone());
        exit(1);
    }

    // 获取 Session_token
    let session_token: String = response.clone().session_token;

    // 检测更新
    if response.clone().should_upgrade == true {
        warn!("该从端需更新, 建议更新至最新版本");
    }

    return (response, node_id, session_token);
}

// 更新
async fn upgrade_bin(mut client: CloudflareSpeedtestClient<Channel>, args: Args, bootstrapres: BootstrapResponse) {
    // 检测是否为最新版本
    if !bootstrapres.should_upgrade {
        info!("该后端为最新版本, 无需更新");
        return;
    } else {
        info!("准备开始更新后端");
    }

    // 检测是否关闭自动更新
    if args.disable_auto_upgrade {
        warn!("该后端版本需更新, 但由于配置了 Disable Auto Upgrade, 不予更新");
        return;
    }

    info!("开始更新后端");

    // 发送请求并获取更新 Url
    let upgrade_message = match client.upgrade(UpgradeRequest {}).await {
        Ok(tmp) => {
            let result = tmp.into_inner();
            if result.success {
                info!("成功获取更新链接: {}", result.message);
                result
            } else {
                error!("无法获取更新链接, 终止更新继续运行: {}", result.message);
                return;
            }
        },
        Err(e) => {
            error!("无法获取更新链接, 终止更新继续运行: {}", e);
            return;
        },
    };

    // 下载文件
    let version_bin = match Client::new().get(format!("{}-{}-{}", upgrade_message.upgrade_url, env::consts::OS, env::consts::ARCH)).send().await { // 构建 URL
        Ok(tmp) => {
            if tmp.status().is_success() {
                tmp
            } else {
                error!("无法下载文件 URL: {}, Code: {}, 终止更新继续运行", tmp.url().to_string(), tmp.status().to_string());
                return ;
            }
        },
        Err(e) => {
            error!("无法下载文件 URL: {}, 终止更新并继续运行: {}", format!("{}-{}-{}", upgrade_message.upgrade_url, env::consts::OS, env::consts::ARCH), e);
            return;
        },
    };

    // 先保存至 tmp dir
    let tmp_dir = env::temp_dir();
    let file_path = tmp_dir.join("CloudflareSpeedtest-Slave");

    match File::create(&file_path) {
        Ok(mut tmp) => {
            let binary = match version_bin.bytes().await {
                Ok(tmp) => tmp,
                Err(e) => {
                    error!("无法获取 Binary, 终止更新并继续运行: {}", e);
                    return;
                },
            };
            match tmp.write_all(&binary) {
                Ok(_) => {
                    info!("成功将 Binary 保存到 Temp Dir");
                },
                Err(e) => {
                    error!("无法将 Binary 保存到 Temp Dir, 终止更新并继续运行: {}", e);
                    return;
                },
            }

        },
        Err(_) => todo!(),
    }

    // 给可执行文件附上可执行权限
    match Command::new("chmod").arg("+x").arg(file_path.clone()).output() {
        Ok(_) => {
            info!("成功添加可执行权限");
        },
        Err(e) => {
            error!("无法添加可执行权限, 终止更新并继续运行: {}", e);
            return;
        },
    }

    // 替换正在运行的可执行文件
    match env::current_exe() {
        Ok(path_to_bin) => {
            match Command::new("cp").arg("-afr").arg(file_path).arg(path_to_bin).output() {
                Ok(tmp) => {
                    if tmp.status.success() {
                        info!("成功将可执行文件替换");
                    } else {
                        error!("无法将可执行文件替换, 终止更新并继续运行");
                        return;
                    }
                },
                Err(e) => {
                    error!("无法将可执行文件替换, 终止更新并继续运行: {}", e);
                    return;
                },
            }
        },
        Err(e) => {
            error!("无法获取当前运行程序路径, 终止更新并继续运行: {}", e);
            return;
        },
    }
    
    // 重启程序
    let mut command = Command::new(env::current_exe().unwrap());
    command.args(env::args().skip(1));

    let _ = match command.spawn() {
        Ok(_) => {
            info!("成功启动新程序");
            exit(1);
        },
        Err(e) => {
            error!("无法启动新程序, 主程序将退出, 请自行重新启动新版本程序: {}", e);
            exit(1);
        },
    };
}

// 发送 Speedtest 信息
async fn send_speedtest(client: CloudflareSpeedtestClient<Channel>, node_id: String, session_token: String) -> Result<(SpeedtestResponse, Vec<String>), Box<dyn Error>> {
    // 构建请求体
    let reqwest: SpeedtestRequest = SpeedtestRequest {
        session_token,
        node_id,
    };

    debug!("SpeedtestRequest Message: {:?}", reqwest);

    // 流传输
    let stream = match client.clone().speedtest(reqwest).await {
        Ok(tmp) => {
            tmp.into_inner()
        },
        Err(e) => {
            return Err(Box::new(e));
        },
    };

    // 只保留第一个
    let mut stream = stream.take(1);
    let response = match stream.next().await {
        Some(tmp) => tmp?,
        None => return Err("无法获取任何 Speedtest 回应".into()),
    };

    debug!("SpeedtestResponse Message: {:?}", response);

    // 返回所有的 IP
    let ip_ranges_ips = ip_cidr_to_ips(response.clone().ip_ranges).await?;

    Ok((response, ip_ranges_ips))
}

// 发送 Speedtest 数据
async fn send_speedtest_result(ip: String, ping: i32, speed: i32, mut client: CloudflareSpeedtestClient<Channel>, node_id: String, session_token: String) -> Result<SpeedtestResultResponse, Box<dyn Error>> {
    // 构建请求体
    let ipresult = IpResult {
        ip_address: ip,
        latency: ping,
        speed,
    };

    // 虽然只有一个, 但也要上个 Vec
    let ipresult_vec: Vec<IpResult> = vec![ipresult];

    // 构建请求体
    let reqwest = SpeedtestResultRequest {
        ip_results: ipresult_vec,
        session_token,
        node_id,
    };

    debug!("SpeedtestResultResponse Message: {:?}", reqwest);

    // 发送信息
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
    // 读取参数
    let args: Args = init_args();
    
    // 判断 Debug 以调整 Log 输出
    if args.debug {
        init_with_level(log::Level::Debug).unwrap();
    } else {
        init_with_level(log::Level::Info).unwrap();
    }
    
    // 判断是否开启 install
    if args.install {
        install_systemd(args);
        exit(1);
    }

    // 真的有人能看到这条消息吗
    if env::consts::OS == "windows" {
        error!("天灭 Windows, Linux/OSX 保平安！");
        error!("由于 fastping-rs 库不支持 Windows, 所以本项目永远不会支持 Windows");
        error!("即使您在 Windows 环境下编译通过, 也绝不可能正常运行！");
        error!("如果您真的需要在 Windows 下运行, 请使用其他项目: 暂无");
        exit(1);
    }

    // 初始化 client
    let client: CloudflareSpeedtestClient<Channel> = init_client(args.clone().server).await;

    // 接下来的是循环的
    loop {
        // 发送 Bootstrap
        let (bootstrap_res, node_id, session_token) = send_bootstrap(client.clone(), args.max_mbps, args.token.clone()).await;

        info!("当前 Node_ID: {}, Session_token: {}", node_id, session_token);

        // 检测 Upgrade
        upgrade_bin(client.clone(), args.clone(), bootstrap_res.clone()).await;

        // 获取需要测试的 IP
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

        // 读取 Speedtest 的 Url、最低速度、最高延迟
        let speedtest_url = speedtest_response.speed_url;
        let speedtest_minimum_mbps = speedtest_response.minimum_mbps;
        let speedtest_maximum_ping = speedtest_response.maximum_ping;

        // 测试 Ping
        let mut ips_ping: std::collections::HashMap<String, u128> = ping_ips(need_ping_ips, speedtest_maximum_ping).await;
        info!("总计 IP 有 {} 个", ips_ping.len());
        debug!("总计 IP: {:?}", ips_ping);
        ips_ping.retain(|_, &mut value| value != u128::MAX);
        info!("符合条件 IP 有 {} 个", ips_ping.len());
        debug!("符合条件 IP: {:?}", ips_ping);

        // 用于储存最后的 IP
        let mut the_last_ip: String = String::new();
        let mut the_last_ip_ping: i32 = -1;
        let mut the_last_ip_speed: i32 = -1;

        // 测试 Ping, 有大于的最低速度的直接返回
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

        // 发送 Speedtest 结果
        match send_speedtest_result(the_last_ip, the_last_ip_ping, the_last_ip_speed, client.clone(), node_id.clone(), session_token.clone()).await {
            Ok(_) => info!("成功完成一次 Speedtest, 开始继续接受 Speedtest 信息"),
            Err(_) => todo!(),
        }
    }
}