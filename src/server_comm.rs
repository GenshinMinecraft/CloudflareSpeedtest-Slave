use std::{error::Error, process::exit, time::Duration};

use crate::{
    cfst_rpc::*, cloudflare_speedtest_client::CloudflareSpeedtestClient, ping::ip_cidr_to_ips,
};

use log::{debug, error, info, warn};
use tonic::transport::{Channel, Endpoint};
use uuid::Uuid;

/**
 * 异步初始化CloudflareSpeedtest客户端。
 *
 * 本函数尝试与指定的服务器建立连接, 并返回一个CloudflareSpeedtestClient实例。
 * 如果连接成功, 它将打印一条成功连接的消息, 并返回客户端实例。
 * 如果连接失败, 它将打印连接错误的消息, 并退出程序。
 *
 * @param server_url 服务器URL, 用于建立连接。
 * @return CloudflareSpeedtestClient实例, 用于后续速度测试操作。
 */
pub async fn init_client(
    server_url: String,
) -> Result<CloudflareSpeedtestClient<Channel>, Box<dyn Error>> {
    // 尝试连接到指定的服务器
    let endpoint = match Endpoint::from_shared("http://".to_string() + &server_url) {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法解析服务器地址: {}", e);
            return Err(Box::new(tonic::Status::aborted("无法解析服务器地址")));
        }
    };

    let client = match CloudflareSpeedtestClient::connect(
        endpoint
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Some(std::time::Duration::from_secs(5)))
            .http2_keep_alive_interval(Duration::from_secs(5))
            .keep_alive_timeout(Duration::from_secs(5))
            .keep_alive_while_idle(true),
    )
    .await
    {
        Ok(tmp) => {
            // 连接成功, 打印成功消息并返回客户端实例
            info!("成功连接服务器");
            tmp
        }
        Err(e) => {
            // 连接失败, 打印错误消息并返回错误
            error!("无法连接服务器: {}", e);
            return Err(Box::new(tonic::Status::aborted("无法连接服务器")));
        }
    };
    return Ok(client);
}

/// 异步发送启动配置请求并处理响应。
///
/// 此函数创建一个唯一的节点ID, 构造一个启动请求, 并使用给定的CloudflareSpeedtestClient发送该请求。
/// 它处理可能的错误, 检查响应是否成功, 并返回相关的响应数据。
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
pub async fn send_bootstrap(
    client: CloudflareSpeedtestClient<Channel>,
    maximum_mbps: i32,
    bootstrap_token: String,
) -> Result<(BootstrapResponse, String, String), Box<dyn Error>> {
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
            return Err(Box::new(tonic::Status::aborted("Bootstrap Boomed!")));
        }
    };

    // 记录响应详情
    debug!("BootStrapResponse Message: {:?}", response);

    // 检查启动请求是否成功
    if response.clone().success != true {
        error!(
            "Bootstrap 信息已成功, 但返回错误 (也许是 Bootstrap Token 设置错误): {:?}",
            response.clone()
        );
        exit(1);
    }

    // 从响应中提取会话令牌
    let session_token: String = response.clone().session_token;

    // 如果响应指示需要升级, 则发出警告
    if response.clone().should_upgrade == true {
        warn!("该后端需更新, 建议更新至最新版本");
    }

    // 返回响应、节点ID和会话令牌
    return Ok((response, node_id, session_token));
}

/// 异步发送速度测试请求到主端, 并返回速度测试响应和IP范围列表。
///
/// 此函数使用提供的Cloudflare Speedtest客户端、节点ID和会话令牌来发起速度测试请求。
/// 它首先构建一个速度测试请求, 然后发送该请求并处理响应。如果请求成功, 它将解析响应中的IP范围,
/// 并将这些信息以及原始响应一起返回。
///
/// 参数:
/// - `client`: 用于与主端通信的客户端。
/// - `node_id`: 用于速度测试的节点ID。
/// - `session_token`: 用于验证会话的令牌。
///
/// 返回值:
/// - `Result<(SpeedtestResponse, Vec<String>), Box<dyn Error>>`: 包含速度测试响应和IP范围列表的结果。
/// 如果发生错误, 返回一个包含错误详情的Box<dyn Error>。
pub async fn send_speedtest(
    client: CloudflareSpeedtestClient<Channel>,
    node_id: String,
    session_token: String,
) -> Result<(SpeedtestResponse, Vec<String>), Box<dyn Error>> {
    // 构建速度测试请求
    let reqwest: SpeedtestRequest = SpeedtestRequest {
        session_token,
        node_id,
    };

    // 在发送请求前, 记录请求的详细信息
    debug!("SpeedtestRequest Message: {:?}", reqwest);

    // 发送速度测试请求并处理响应
    let mut stream = match client.clone().speedtest(reqwest).await {
        Ok(tmp) => tmp.into_inner(),
        Err(e) => return Err(Box::new(e)),
    };

    // 该代码解决了无法检测是否与主端断开连接的问题
    // 让我们感谢 Moohr!
    // 尝试读取流中的消息并处理可能的错误
    loop {
        match stream.message().await {
            Ok(Some(response)) => {
                // Process the valid response message here
                debug!("SpeedtestResponse Message: {:?}", response);

                // Convert IP ranges to specific IP addresses list
                let ip_ranges_ips = ip_cidr_to_ips(response.clone().ip_ranges).await?;

                // Exit the loop after processing the message or continue as needed
                return Ok((response, ip_ranges_ips));
            }
            Ok(None) => {
                // The stream was closed by the sender
                error!("与主端的流传输被迫关闭, 这可能是因为后端网络波动或主端崩溃, 正在尝试重新连接...");
                // Handle the closure of the stream, possibly attempt to reconnect or exit
                return Err(Box::new(tonic::Status::aborted(
                    "Stream was closed by the sender.",
                )));
            }
            Err(e) => {
                // An error occurred while fetching the next message
                error!("无法接收主端发送的消息, 正在尝试重新连接: {}", e);
                return Err(Box::new(e));
            }
        }
    }
}

/// 异步发送速度测试结果到主端。
///
/// 此函数接收速度测试的IP地址、ping值和速度, 以及一个Cloudflare速度测试客户端,
/// 用于向主端发送速度测试结果。它还接收一个节点ID和会话令牌, 这些可能是用于
/// 鉴权或标识测试来源的。
///
/// 返回结果为速度测试响应, 或者一个错误盒子。如果成功发送了测试结果, 它将返回测试结果的副本。
pub async fn send_speedtest_result(
    ip: String,
    ping: i32,
    speed: i32,
    mut client: CloudflareSpeedtestClient<Channel>,
    node_id: String,
    session_token: String,
) -> Result<SpeedtestResultResponse, Box<dyn Error>> {
    // 构建IP结果对象, 包含IP地址、延迟和速度信息。
    let ipresult = IpResult {
        ip_address: ip,
        latency: ping,
        speed,
    };

    // 将IP结果对象封装成一个vector, 以满足请求格式的要求。
    let ipresult_vec: Vec<IpResult> = vec![ipresult];

    // 构建速度测试结果请求, 包含IP结果、会话令牌和节点ID。
    let reqwest = SpeedtestResultRequest {
        ip_results: ipresult_vec,
        session_token,
        node_id,
    };

    // 打印调试信息, 显示即将发送的速度测试结果请求。
    debug!("SpeedtestResultResponse Message: {:?}", reqwest);

    // 尝试发送速度测试结果请求, 并处理结果。
    match client.speedtest_result(reqwest).await {
        Ok(tmp) => {
            // 如果发送成功, 记录信息并返回结果的副本。
            info!("成功发送 Speedtest Result 信息");
            return Ok(tmp.get_ref().clone());
        }
        Err(e) => {
            // 如果发送失败, 记录错误并返回错误盒子。
            error!("无法发送 Speedtest Result 信息: {}", e);
            return Err(Box::new(e));
        }
    }
}
