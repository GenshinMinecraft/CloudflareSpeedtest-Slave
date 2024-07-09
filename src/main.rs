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

/**
 * 初始化程序的参数对象。
 *
 * 该函数通过解析命令行参数，创建并返回一个Args对象。
 * Args对象包含了程序运行时的所有配置参数，这些参数可以通过命令行进行定制。
 * 
 * 返回值:
 * Args - 一个包含了程序运行参数的数据结构。
 */
fn init_args() -> Args {
    // 使用Args::parse方法从命令行参数中构建Args对象。
    let args: Args = Args::parse();
    // 返回构建好的Args对象。
    return args;
}

/// 安装并配置 Systemd 服务。
/// 
/// 此函数检查当前系统是否为 Linux，并确认是否使用 Systemd 作为服务管理器。
/// 它还需要以 root 用户身份运行，以复制可执行文件并修改系统服务配置。
/// 最后，它将根据提供的参数配置并启动一个名为 cfst_slave.service 的 Systemd 服务。
fn install_systemd(args: Args) {
    // 检查操作系统是否为 Linux
    if env::consts::OS != "linux" {
        error!("Install 功能仅适用于 Linux 系统");
        exit(1);
    }
    
    // 检查系统是否使用 Systemd
    match fs::metadata("/usr/bin/systemctl") {
        Ok(_) => {
            info!("您的系统使用的是 Systemd 服务管理器, 可以正常使用 Install 功能")
        },
        Err(_) => {
            error!("您的系统并非使用 Systemd 作为服务管理器, 无法使用 Install 功能, 请自行配置进程守护");
            exit(1);
        },
    }

    // 确认以 root 用户身份运行
    if std::env::var("USER") == Ok("root".to_string()) {
        info!("正在使用 root 用户");
    } else {
        error!("非 root 用户, 请使用 root 用户运行 Install 功能");
        exit(1);
    }

    // 检查是否已存在相同名称的服务文件
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

    // 复制可执行文件到 /usr/bin
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

    // 配置服务文件的内容
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

    // 根据参数替换服务文件中的占位符
    let mut replaced_service_config = service_config.replace("SERVERURL", args.server.as_str());
    replaced_service_config = replaced_service_config.replace("TOKEN", args.token.as_str());
    if args.debug {
        let tmp = args.max_mbps.to_string() + " --debug";
        replaced_service_config = replaced_service_config.replace("MAXMBPS", tmp.as_str());
    } else {
        replaced_service_config = replaced_service_config.replace("MAXMBPS", args.max_mbps.to_string().as_str());
    }

    // 删除旧的服务文件
    match Command::new("rm").arg("-rf").arg("/etc/systemd/system/cfst_slave.service").output() {
        Ok(_) => {},
        Err(e) => {
            error!("无法删除 /etc/systemd/system/cfst_slave.service: {}", e);
            exit(1);
        },
    }

    // 创建并写入新的服务文件
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
    
    // 重新加载 Systemd 配置
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

    // 启动服务
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

    // 询问用户是否开启开机自启动
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

/**
 * 异步初始化CloudflareSpeedtest客户端。
 * 
 * 本函数尝试与指定的服务器建立连接，并返回一个CloudflareSpeedtestClient实例。
 * 如果连接成功，它将打印一条成功连接的消息，并返回客户端实例。
 * 如果连接失败，它将打印连接错误的消息，并退出程序。
 * 
 * @param server_url 服务器URL，用于建立连接。
 * @return CloudflareSpeedtestClient实例，用于后续速度测试操作。
 */
async fn init_client(server_url: String) -> CloudflareSpeedtestClient<Channel> {
    // 尝试连接到指定的服务器
    let client = match CloudflareSpeedtestClient::connect("http://".to_string() + &server_url).await {
        Ok(tmp) => {
            // 连接成功，打印成功消息并返回客户端实例
            info!("成功连接服务器");
            tmp
        },
        Err(e) => {
            // 连接失败，打印错误消息并退出程序
            error!("无法连接服务器: {}", e);
            exit(1);
        },
    };
    return client;
}

/// 异步发送启动配置请求并处理响应。
///
/// 此函数创建一个唯一的节点ID，构造一个启动请求，并使用给定的CloudflareSpeedtestClient发送该请求。
/// 它处理可能的错误，检查响应是否成功，并返回相关的响应数据。
///
/// 参数:
/// - client: 用于发送启动请求的CloudflareSpeedtestClient实例。
/// - maximum_mbps: 测试允许的最大Mbps值。
/// - bootstrap_token: 用于身份验证的启动令牌。
///
/// 返回:
/// - BootstrapResponse: 启动请求的响应。
/// - String: 生成的节点ID。
/// - String: 响应中的会话令牌。
async fn send_bootstrap(client: CloudflareSpeedtestClient<Channel>, maximum_mbps: i32, bootstrap_token: String) -> (BootstrapResponse, String, String) {
    // 生成一个唯一的节点ID
    let node_id: String = Uuid::new_v4().to_string();
    
    // 构造启动请求对象
    let reqwest: BootstrapRequest = BootstrapRequest {
        maximum_mbps: maximum_mbps,
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        bootstrap_token: bootstrap_token,
        node_id: node_id.clone(),
    };

    // 在发送请求前记录请求详情
    debug!("BootStrapRequest Message: {:?}", reqwest);

    // 尝试发送启动请求并处理可能的错误
    let response: BootstrapResponse = match client.clone().bootstrap(reqwest).await {
        Ok(res) => res.get_ref().clone(),
        Err(e) => {
            error!("发送 Bootstrap 时发送错误: {}", e);
            exit(1);
        },
    };

    // 记录响应详情
    debug!("BootStrapResponse Message: {:?}", response);

    // 检查启动请求是否成功
    if response.clone().success != true {
        error!("Bootstrap 信息已成功, 但返回错误 (也许是 Bootstrap Token 设置错误): {:?}", response.clone());
        exit(1);
    }

    // 从响应中提取会话令牌
    let session_token: String = response.clone().session_token;

    // 如果响应指示需要升级，则发出警告
    if response.clone().should_upgrade == true {
        warn!("该从端需更新, 建议更新至最新版本");
    }

    // 返回响应、节点ID和会话令牌
    return (response, node_id, session_token);
}

// 异步函数，负责检查并执行云flare速度测试客户端的更新。
// 参数:
// - client: 云flare速度测试客户端实例，使用channel进行通信。
// - args: 命令行参数，包含是否禁用自动升级等信息。
// - bootstrapres: 启动时从服务器获取的响应，包含是否需要升级的信息。
async fn upgrade_bin(mut client: CloudflareSpeedtestClient<Channel>, args: Args, bootstrapres: BootstrapResponse) {
    // 检查是否需要升级，如果不需升级则直接返回。
    if !bootstrapres.should_upgrade {
        info!("该后端为最新版本, 无需更新");
        return;
    } else {
        info!("准备开始更新后端");
    }

    // 如果配置了禁用自动升级，则即使需要升级也不执行更新。
    if args.disable_auto_upgrade {
        warn!("该后端版本需更新, 但由于配置了 Disable Auto Upgrade, 不予更新");
        return;
    }

    info!("开始更新后端");

    // 尝试从客户端获取更新信息。
    let upgrade_message = match client.upgrade(UpgradeRequest {}).await {
        Ok(tmp) => {
            let result = tmp.into_inner();
            // 如果更新请求成功，检查是否成功获取了更新链接。
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

    // 根据更新信息下载对应的操作系统和架构的更新文件。
    let version_bin = match Client::new().get(format!("{}-{}-{}", upgrade_message.upgrade_url, env::consts::OS, env::consts::ARCH)).send().await {
        Ok(tmp) => {
            // 检查下载是否成功。
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

    // 在临时目录创建一个文件用于存储下载的更新二进制文件。
    let tmp_dir = env::temp_dir();
    let file_path = tmp_dir.join("CloudflareSpeedtest-Slave");

    match File::create(&file_path) {
        Ok(mut tmp) => {
            // 下载二进制文件内容。
            let binary = match version_bin.bytes().await {
                Ok(tmp) => tmp,
                Err(e) => {
                    error!("无法获取 Binary, 终止更新并继续运行: {}", e);
                    return;
                },
            };
            // 将二进制文件内容写入到临时文件。
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

    // 为临时文件添加可执行权限。
    match Command::new("chmod").arg("+x").arg(file_path.clone()).output() {
        Ok(_) => {
            info!("成功添加可执行权限");
        },
        Err(e) => {
            error!("无法添加可执行权限, 终止更新并继续运行: {}", e);
            return;
        },
    }

    // 复制临时文件到当前执行程序的路径，以替换旧版本。
    match env::current_exe() {
        Ok(path_to_bin) => {
            match Command::new("cp").arg("-afr").arg(file_path).arg(path_to_bin).output() {
                Ok(tmp) => {
                    // 检查复制是否成功。
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
    
    // 启动新的可执行文件，替换当前进程。
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

/// 异步发送速度测试请求到主端，并返回速度测试响应和IP范围列表。
///
/// 此函数使用提供的Cloudflare Speedtest客户端、节点ID和会话令牌来发起速度测试请求。
/// 它首先构建一个速度测试请求，然后发送该请求并处理响应。如果请求成功，它将解析响应中的IP范围，
/// 并将这些信息以及原始响应一起返回。
///
/// 参数:
/// - `client`: 用于与主端通信的客户端。
/// - `node_id`: 用于速度测试的节点ID。
/// - `session_token`: 用于验证会话的令牌。
///
/// 返回值:
/// - `Result<(SpeedtestResponse, Vec<String>), Box<dyn Error>>`: 包含速度测试响应和IP范围列表的结果。
/// 如果发生错误，返回一个包含错误详情的Box<dyn Error>。
async fn send_speedtest(
    client: CloudflareSpeedtestClient<Channel>,
    node_id: String,
    session_token: String,
) -> Result<(SpeedtestResponse, Vec<String>), Box<dyn Error>> {
    // 构建速度测试请求
    let reqwest: SpeedtestRequest = SpeedtestRequest {
        session_token,
        node_id,
    };

    // 在发送请求前，记录请求的详细信息
    debug!("SpeedtestRequest Message: {:?}", reqwest);

    // 发送速度测试请求并处理响应
    let stream = match client.clone().speedtest(reqwest).await {
        Ok(tmp) => tmp.into_inner(),
        Err(e) => return Err(Box::new(e)),
    };

    // 限制流中的项目数量为1，因为我们只期望一个响应
    let mut stream = stream.take(1);
    // 从流中获取下一个项目（即响应），如果没有项目，则返回错误
    let response = match stream.next().await {
        Some(tmp) => tmp?,
        None => return Err("无法获取任何 Speedtest 回应".into()),
    };

    // 在接收到响应后，记录响应的详细信息
    debug!("SpeedtestResponse Message: {:?}", response);

    // 将IP范围转换为具体的IP地址列表
    let ip_ranges_ips = ip_cidr_to_ips(response.clone().ip_ranges).await?;

    // 返回速度测试响应和IP地址列表
    Ok((response, ip_ranges_ips))
}

/// 异步发送速度测试结果到主端。
///
/// 此函数接收速度测试的IP地址、ping值和速度，以及一个Cloudflare速度测试客户端，
/// 用于向主端发送速度测试结果。它还接收一个节点ID和会话令牌，这些可能是用于
/// 鉴权或标识测试来源的。
///
/// 返回结果为速度测试响应，或者一个错误盒子。如果成功发送了测试结果，它将返回测试结果的副本。
async fn send_speedtest_result(
    ip: String, 
    ping: i32, 
    speed: i32, 
    mut client: CloudflareSpeedtestClient<Channel>, 
    node_id: String, 
    session_token: String
) -> Result<SpeedtestResultResponse, Box<dyn Error>> {
    // 构建IP结果对象，包含IP地址、延迟和速度信息。
    let ipresult = IpResult {
        ip_address: ip,
        latency: ping,
        speed,
    };

    // 将IP结果对象封装成一个vector，以满足请求格式的要求。
    let ipresult_vec: Vec<IpResult> = vec![ipresult];

    // 构建速度测试结果请求，包含IP结果、会话令牌和节点ID。
    let reqwest = SpeedtestResultRequest {
        ip_results: ipresult_vec,
        session_token,
        node_id,
    };

    // 打印调试信息，显示即将发送的速度测试结果请求。
    debug!("SpeedtestResultResponse Message: {:?}", reqwest);

    // 尝试发送速度测试结果请求，并处理结果。
    match client.speedtest_result(reqwest).await {
        Ok(tmp) => {
            // 如果发送成功，记录信息并返回结果的副本。
            info!("成功发送 Speedtest Result 信息");
            return Ok(tmp.get_ref().clone());
        },
        Err(e) => {
            // 如果发送失败，记录错误并返回错误盒子。
            error!("无法发送 Speedtest Result 信息: {}", e);
            return Err(Box::new(e));
        },
    }
}


#[tokio::main]
async fn main() {
    // 初始化命令行参数
    let args: Args = init_args();
    
    // 根据调试模式设置日志级别
    if args.debug {
        init_with_level(log::Level::Debug).unwrap();
    } else {
        init_with_level(log::Level::Info).unwrap();
    }
    
    // 如果命令行参数包含安装选项，则执行安装操作并退出
    if args.install {
        install_systemd(args);
        exit(1);
    }

    // 检查操作系统是否为Windows，如果是，则输出错误信息并退出
    if env::consts::OS == "windows" {
        error!("天灭 Windows, Linux/OSX 保平安！");
        error!("由于 fastping-rs 库不支持 Windows, 所以本项目永远不会支持 Windows");
        error!("即使您在 Windows 环境下编译通过, 也绝不可能正常运行！");
        error!("如果您真的需要在 Windows 下运行, 请使用其他项目: 暂无");
        exit(1);
    }

    // 初始化Cloudflare Speedtest客户端
    let client: CloudflareSpeedtestClient<Channel> = init_client(args.clone().server).await;

    // 主循环，用于定期执行速度测试
    loop {
        // 发送启动请求，获取节点ID和会话令牌
        let (bootstrap_res, node_id, session_token) = send_bootstrap(client.clone(), args.max_mbps, args.token.clone()).await;

        // 日志记录当前节点ID和会话令牌
        info!("当前 Node_ID: {}, Session_token: {}", node_id, session_token);

        // 升级客户端二进制文件
        upgrade_bin(client.clone(), args.clone(), bootstrap_res.clone()).await;

        // 发送速度测试请求，获取测试结果和需要ping的IP列表
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

        // 对需要ping的IP进行ping测试，记录延迟
        let mut ips_ping: std::collections::HashMap<String, u128> = ping_ips(need_ping_ips, speedtest_response.maximum_ping).await;
        info!("总计 IP 有 {} 个", ips_ping.len());
        debug!("总计 IP: {:?}", ips_ping);
        // 移除延迟过高的IP
        ips_ping.retain(|_, &mut value| value != u128::MAX);
        info!("符合条件 IP 有 {} 个", ips_ping.len());
        debug!("符合条件 IP: {:?}", ips_ping);

        // 测试每个IP的速度，选择最快且符合最小速度要求的IP
        let mut the_last_ip: String = String::new();
        let mut the_last_ip_ping: i32 = -1;
        let mut the_last_ip_speed: i32 = -1;

        for (speed_ip, _) in ips_ping.clone() {
            let tmp_speed = speed_one_ip(speedtest_response.speed_url.clone(), speed_ip.clone(), 10).await;
            if tmp_speed.round() as i32 >= speedtest_response.minimum_mbps {
                the_last_ip = speed_ip;
                the_last_ip_ping = *ips_ping.get(&the_last_ip).unwrap() as i32;
                the_last_ip_speed = tmp_speed.round() as i32;
                break;
            } else {
                continue;
            }
        }

        // 发送速度测试结果
        match send_speedtest_result(the_last_ip, the_last_ip_ping, the_last_ip_speed, client.clone(), node_id.clone(), session_token.clone()).await {
            Ok(_) => info!("成功完成一次 Speedtest, 开始继续接受 Speedtest 信息"),
            Err(_) => todo!(),
        }
    }
}