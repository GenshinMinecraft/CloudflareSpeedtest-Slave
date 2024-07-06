use std::collections::HashMap;
use fastping_rs::PingResult::{Idle, Receive};
use fastping_rs::Pinger;
use log::{debug, error};
use std::error::Error;
use ipnetwork::IpNetwork;

pub async fn ping_ips(ips: Vec<String>, maximum_ping: i32) -> HashMap<String, u128> {
    let (pinger, results) = match Pinger::new(Some(maximum_ping as u64), Some(56)) {
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

    let mut ips_rtt_map: HashMap<String, u128> = HashMap::new();

    loop {
        match results.recv() {
            Ok(result) => match result {
                Idle { addr } => {
                    debug!("无效/不可达的 IP:  {}.", addr);
                    ips_rtt_map.insert(addr.to_string(), u128::MAX);
                    
                    if ips_rtt_map.len() == ips.len() {
                        pinger.stop_pinger();
                        break;
                    }
                }
                Receive { addr, rtt } => {
                    debug!("存活 IP: {} in {:?}.", addr, rtt);
                    ips_rtt_map.insert(addr.to_string(), rtt.as_millis());

                    if ips_rtt_map.len() == ips.len() {
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

    return ips_rtt_map;
}


pub async fn ip_cidr_to_ips(ip_cidr: Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
    let ip_cidr_string: Vec<String> = ip_cidr.into_iter().map(|fs| fs.to_string()).collect(); 

    let mut ip_addresses: Vec<String> = Vec::new();

    for ips in ip_cidr_string {
        let network= ips.parse::<IpNetwork>()?;
        for single_ip in network.iter() {
            ip_addresses.push(single_ip.to_string());
        }
    }

    Ok(ip_addresses)
}
