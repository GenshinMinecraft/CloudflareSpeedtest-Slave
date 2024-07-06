use std::time::Duration;
use fastping_rs::PingResult::{Idle, Receive};
use fastping_rs::Pinger;
use log::{info, warn, error};
use faststr::FastStr;
use std::error::Error;
use ipnetwork::IpNetwork;

fn duration_to_f64(duration: Duration) -> f64 {
    // 获取整个秒数
    let seconds = duration.as_secs() as f64;
    let nanos = duration.subsec_nanos() as f64 / 1e9;
    return seconds + nanos;
}

pub async fn ping_ips(ips: Vec<String>) -> Vec<f64>{
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


pub async fn ip_cidr_to_ips(ip_cidr: Vec<FastStr>) -> Result<Vec<String>, Box<dyn Error>> {
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
