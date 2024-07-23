mod args;
mod cfst_rpc;
mod install_upgrade;
mod ping;
mod server_comm;
mod speed;

use crate::{args::*, cfst_rpc::*, install_upgrade::*, ping::*, server_comm::*, speed::*};

use cloudflare_speedtest_client::CloudflareSpeedtestClient;
use log::{debug, error, info, warn};
use rustls::crypto::aws_lc_rs;
use simple_logger::init_with_level;
use std::{process::exit, time::Duration};
use tonic::transport::Channel;

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

    if args.max_mbps == 114514 {
        error!("必须设置 Max Mbps 参数: -m / --max-mbps ");
        exit(1);
    }

    // 如果命令行参数包含安装选项, 则执行安装操作并退出
    if args.install {
        install_systemd(args);
        exit(1);
    }

    let _ = aws_lc_rs::default_provider().install_default().unwrap();

    // 主循环, 用于定期执行速度测试
    loop {
        // 初始化Cloudflare Speedtest客户端
        let client: CloudflareSpeedtestClient<Channel> =
            match init_client(args.clone().server).await {
                Ok(tmp) => {
                    info!("成功初始化 Cloudflare Speedtest 客户端");
                    tmp
                }
                Err(e) => {
                    error!(
                        "未能成功初始化 Cloudflare Speedtest 客户端, 15sec 后重新连接服务器: {}",
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
            };

        // 发送启动请求, 获取节点ID和会话令牌
        let (bootstrap_res, node_id, session_token) =
            match send_bootstrap(client.clone(), args.max_mbps, args.token.clone()).await {
                Ok(tmp) => {
                    info!("成功获取 Bootstrap 信息");
                    tmp
                }
                Err(e) => {
                    error!("未能成功获取 Bootstrap 信息, 15sec 后重新连接服务器: {}", e);
                    tokio::time::sleep(Duration::from_secs(15)).await;
                    continue;
                }
            };

        // 日志记录当前节点ID和会话令牌
        info!(
            "当前 Node_ID: {}, Session_token: {}",
            node_id, session_token
        );

        // 升级客户端二进制文件
        upgrade_bin(client.clone(), args.clone(), bootstrap_res.clone()).await;

        loop {
            // 发送速度测试请求, 获取测试结果和需要ping的IP列表
            let (speedtest_response, need_ping_ips) = match send_speedtest(
                client.clone(),
                node_id.clone(),
                session_token.clone(),
            )
            .await
            {
                Ok((res, str)) => {
                    info!("成功获取 Speedtest 信息, 开始启动测速程序");
                    (res, str)
                }
                Err(e) => {
                    error!("未能成功获取需要测试的 IP, 正在重新连接服务器: {}", e);
                    break;
                }
            };

            // 对需要ping的IP进行ping测试, 记录延迟
            let mut ips_ping: std::collections::HashMap<String, u128> =
                ping_ips(need_ping_ips, speedtest_response.maximum_ping).await;
            info!("获取到 {} 个 IP, 开始测试", ips_ping.len());
            // 移除延迟过高的IP
            ips_ping.retain(|_, &mut value| value != u128::MAX);
            info!("符合条件 IP 有 {} 个", ips_ping.len());
            debug!("符合条件 IP: {:?}", ips_ping);

            // 测试每个IP的速度, 选择最快且符合最小速度要求的IP
            let mut the_last_ip: String = String::new();
            let mut the_last_ip_ping: i32 = -1;
            let mut the_last_ip_speed: i32 = -1;

            for (speed_ip, ping) in ips_ping.clone() {
                let tmp_speed =
                    speed_one_ip(speedtest_response.speed_url.clone(), speed_ip.clone(), 10).await;
                if tmp_speed.round() as i32 >= speedtest_response.minimum_mbps {
                    the_last_ip = speed_ip;
                    the_last_ip_ping = ping as i32;
                    the_last_ip_speed = tmp_speed.round() as i32;
                    break;
                } else {
                    continue;
                }
            }

            if the_last_ip.is_empty() {
                warn!("在测试完所有的 IP 后, 没有发现符合条件的 IP, 请检查您的网络环境, 或请求主端提供者降低最小带宽要求与 Ping 要求");
            }

            // 发送速度测试结果
            match send_speedtest_result(
                the_last_ip,
                the_last_ip_ping,
                the_last_ip_speed,
                client.clone(),
                node_id.clone(),
                session_token.clone(),
            )
            .await
            {
                Ok(_) => info!("成功完成一次 Speedtest, 开始继续接受 Speedtest 信息"),
                Err(e) => {
                    error!("无法发送测试结果, 将会跳过本次测试: {}", e);
                    continue;
                }
            }
        }
    }
}
