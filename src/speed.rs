use std::{
    io::{Read, Write}, net::TcpStream, time::Instant
};

use log::{error, info};
use url::Url;
use native_tls::TlsConnector;

pub async fn speed_one_ip(speedtest_url: String, ip: String, speed_time: u32) -> f64 {

    info!("开始测速");
    
    let url = match Url::parse(speedtest_url.as_str()) {
        Ok(parsed_url) => parsed_url,
        Err(_) => {
            error!("无法正确解析 Speedtest URL");
            return -1.0;
        },
    };

    let domain = match url.domain() {
        Some(tmp) => tmp,
        None => {
            error!("无法获取测速链接中的域名");
            return -1.0;
        },
    };
    let port = url.port().unwrap_or(443);
    let path = url.path();

    let stream = match TcpStream::connect(format!("{}:{}", ip, port)) {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法初始化 TcpStream: {}", e);
            return -1.0;
        },
    };
    let connector = match TlsConnector::builder().build() {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法初始化 TlsConnector: {}", e);
            return  -1.0;
        },
    };
    let mut stream = match connector.connect(domain, stream) {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法初始化 TlsStream: {}", e);
            return -1.0;
        },
    };
    
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, domain
    );

    let start_time = Instant::now();
    let mut buffer = [0; 10240];
    let mut data = 0;

    match stream.write_all(request.as_bytes()) {
        Ok(_) => {
            
        },
        Err(e) => {
            error!("无法发送请求: {}", e);
            return -1.0;
        },
    }

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break, 
            Ok(n) => {
                data += n;
                
                if start_time.elapsed().as_secs_f64() >= speed_time as f64 {
                    break;
                }
            },
            Err(_e) => break,
        }
    }

    let bytes_downloaded = data;
    let time_taken: f64 = start_time.elapsed().as_secs_f64();


    let bits_downloaded: f64 = bytes_downloaded as f64 * 8.0; 
    let download_speed_bps: f64 = bits_downloaded / time_taken; 

    let download_speed_kbps: f64 = download_speed_bps / 1000.0;
    let download_speed_mbps: f64 = download_speed_kbps / 1000.0;

    info!("IP: {}, 速度: {}mbps", ip, download_speed_mbps);

    download_speed_mbps
}