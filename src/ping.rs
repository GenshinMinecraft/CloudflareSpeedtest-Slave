use std::{collections::HashMap, error::Error, process::exit};

use fastping_rs::{
    PingResult::{Idle, Receive},
    Pinger,
};
use ipnetwork::IpNetwork;
use log::{debug, error};

/// 异步发送ping请求到指定的IP地址列表, 并返回每个地址的响应时间。
///
/// 参数:
/// - ips: IP地址字符串的向量, 这些地址将被ping测试。
/// - maximum_ping: 允许的最大ping响应时间（以毫秒为单位）。
///
/// 返回值:
/// 一个HashMap, 其中键是IP地址, 值是对应的ping响应时间（以毫秒为单位）。如果IP地址没有响应, 则值为u128的最大值。
pub async fn ping_ips(ips: Vec<String>, maximum_ping: i32) -> HashMap<String, u128> {
    // 初始化Pinger实体, 用于实际的ping测试。
    let (pinger, results) = match Pinger::new(Some(maximum_ping as u64), Some(56)) {
        Ok((pinger, results)) => (pinger, results),
        Err(e) => {
            // 日志记录初始化失败的错误, 并panic。
            error!("新建 Pinger 时候出错, 这可能是因为您未使用 Root 权限运行或未添加创建原始套接字的权限, 详情请看 https://github.com/GenshinMinecraft/CloudflareSpeedtest-Slave?tab=readme-ov-file#warning : {}", e);
            exit(1);
        }
    };

    // 向pinger添加待测试的IP地址。
    for ip in ips.clone() {
        pinger.add_ipaddr(&ip);
    }

    // 启动ping测试。
    pinger.run_pinger();

    // 初始化用于存储IP地址和响应时间的结果映射。
    let mut ips_rtt_map: HashMap<String, u128> = HashMap::new();

    // 不断接收ping测试结果, 直到所有IP地址都测试完毕。
    loop {
        match results.recv() {
            Ok(result) => match result {
                // 如果某个IP地址没有响应, 将其添加到结果映射中, 值设为最大值。
                Idle { addr } => {
                    debug!("无效/不可达的 IP:  {}.", addr);
                    ips_rtt_map.insert(addr.to_string(), u128::MAX);

                    // 如果所有IP地址都已测试完毕, 停止ping测试。
                    if ips_rtt_map.len() == ips.len() {
                        pinger.stop_pinger();
                        break;
                    }
                }
                // 如果某个IP地址有响应, 记录其响应时间。
                Receive { addr, rtt } => {
                    debug!("存活 IP: {} in {:?}.", addr, rtt);
                    ips_rtt_map.insert(addr.to_string(), rtt.as_millis());

                    // 如果所有IP地址都已测试完毕, 停止ping测试。
                    if ips_rtt_map.len() == ips.len() {
                        pinger.stop_pinger();
                        break;
                    }
                }
            },
            // 如果接收结果时发生错误, 进行日志记录。
            Err(e) => {
                error!("获取 IP 测试结果时出现错误: {}", e);
            }
        }
    }

    // 返回所有IP地址的测试结果。
    return ips_rtt_map;
}

pub async fn ip_cidr_to_ips(ip_cidr: Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
    let ip_cidr_string: Vec<String> = ip_cidr.into_iter().map(|fs| fs.to_string()).collect();

    let mut ip_addresses: Vec<String> = Vec::new();

    for ips in ip_cidr_string {
        let network = ips.parse::<IpNetwork>()?;
        for single_ip in network.iter() {
            ip_addresses.push(single_ip.to_string());
        }
    }

    Ok(ip_addresses)
}
