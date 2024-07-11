mod ping;
mod speed;
mod cfst_rpc;
mod install_upgrade;
mod args;
mod server_comm;

use crate::{
    args::*,
    cfst_rpc::*,
    install_upgrade::*,
    ping::*,
    server_comm::*,
    speed::*,
};

use std::{env, process::exit};
use cloudflare_speedtest_client::CloudflareSpeedtestClient;
use log::{debug, error, info, warn};
use rustls::crypto::aws_lc_rs;
use simple_logger::init_with_level;
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
    
    // 如果命令行参数包含安装选项, 则执行安装操作并退出
    if args.install {
        install_systemd(args);
        exit(1);
    }

    // 检查操作系统是否为Windows, 如果是, 则输出错误信息并退出
    if env::consts::OS == "windows" {
        error!("天灭 Windows, Linux/OSX 保平安！");
        error!("由于 fastping-rs 库不支持 Windows, 所以本项目永远不会支持 Windows");
        error!("即使您在 Windows 环境下编译通过, 也绝不可能正常运行！");
        error!("如果您真的需要在 Windows 下运行, 请使用其他项目: 暂无");
        exit(1);
    }

    // 初始化Cloudflare Speedtest客户端
    let client: CloudflareSpeedtestClient<Channel> = init_client(args.clone().server).await;

    let _ = aws_lc_rs::default_provider().install_default().unwrap();

    // 主循环, 用于定期执行速度测试
    loop {
        // 发送启动请求, 获取节点ID和会话令牌
        let (bootstrap_res, node_id, session_token) = send_bootstrap(client.clone(), args.max_mbps, args.token.clone()).await;

        // 日志记录当前节点ID和会话令牌
        info!("当前 Node_ID: {}, Session_token: {}", node_id, session_token);

        // 升级客户端二进制文件
        upgrade_bin(client.clone(), args.clone(), bootstrap_res.clone()).await;

        loop {
            // 发送速度测试请求, 获取测试结果和需要ping的IP列表
            let (speedtest_response, need_ping_ips) = match send_speedtest(client.clone(), node_id.clone(), session_token.clone()).await {
                Ok((res, str)) => {
                    info!("成功获取 Speedtest 信息, 开始启动测速程序");
                    (res, str)
                },
                Err(e) => {
                    error!("未能成功获取需要测试的 IP, 正在重新连接服务器: {}", e);
                    break;
                },
            };

            // 对需要ping的IP进行ping测试, 记录延迟
            let mut ips_ping: std::collections::HashMap<String, u128> = ping_ips(need_ping_ips, speedtest_response.maximum_ping).await;
            info!("总计 IP 有 {} 个", ips_ping.len());
            debug!("总计 IP: {:?}", ips_ping);
            // 移除延迟过高的IP
            ips_ping.retain(|_, &mut value| value != u128::MAX);
            info!("符合条件 IP 有 {} 个", ips_ping.len());
            debug!("符合条件 IP: {:?}", ips_ping);

            // 测试每个IP的速度, 选择最快且符合最小速度要求的IP
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

            if the_last_ip.is_empty() {
                warn!("在测试完所有的 IP 后, 没有发现符合条件的 IP, 请检查您的网络环境, 或请求主端提供者降低最小带宽要求与 Ping 要求");
            }

            // 发送速度测试结果
            match send_speedtest_result(the_last_ip, the_last_ip_ping, the_last_ip_speed, client.clone(), node_id.clone(), session_token.clone()).await {
                Ok(_) => info!("成功完成一次 Speedtest, 开始继续接受 Speedtest 信息"),
                Err(e) => {
                    error!("无法发送测试结果, 将会跳过本次测试: {}", e);
                    continue;
                },
            }
        }
    }
}