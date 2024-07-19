use std::time::Duration;
use std::{collections::HashMap, error::Error};

// use clap::error::ContextValue::String;
use futures::stream::iter;
use futures::StreamExt;
use ipnetwork::IpNetwork;
use log::debug;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{timeout, Instant};

async fn ping_single_ip(ip: String, timeout_ms: i32) -> i32 {
    let addr = format!("{}:80", ip);
    let time_out = Duration::from_millis(timeout_ms as u64);
    let start = Instant::now();
    match timeout(time_out, TcpStream::connect(&addr)).await {
        Ok(_) => {
            let duration = start.elapsed().as_millis() as i32;
            if duration <= 10 {
                -1
            } else {
                duration
            }
        }
        Err(_) => -1,
    }
}

pub async fn ping_ips(ips: Vec<String>, maximum_ping: i32) -> HashMap<String, u128> {
    let ip_and_ping_map = std::sync::Arc::new(Mutex::new(HashMap::new()));
    iter(ips)
        .for_each_concurrent(Some(100), |ip| {
            let clone_map = ip_and_ping_map.clone();
            async move {
                let duration = ping_single_ip(ip.clone(), maximum_ping).await;
                if duration != -1 {
                    debug!("IP {} Ping {}ms", ip, duration);
                    let mut map_lock = clone_map.lock().await;
                    map_lock.insert(ip, duration as u128);
                } else {
                    debug!("IP {} 不可达", ip);
                    let mut map_lock = clone_map.lock().await;
                    map_lock.insert(ip, u128::MAX);
                }
            }
        })
        .await;
    let mut inner_map = ip_and_ping_map.lock().await;
    std::mem::take(&mut *inner_map)
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
