use std::{
    io::{Read, Write}, net::TcpStream, time::Instant
};

use log::{error, info};
use url::Url;
use std::sync::Arc;

use rustls::RootCertStore;

pub async fn speed_one_ip(speedtest_url: String, ip: String, speed_time: u32) -> f64 {
    info!("开始测速 IP: {}, URL: {}", ip, speedtest_url);
    
    // 设置 CA 证书
    let root_store = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.into(),
    };

    // 创建 Rustls Config
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // 解析 Url
    let url = match Url::parse(speedtest_url.as_str()) {
        Ok(parsed_url) => parsed_url,
        Err(_) => {
            error!("无法正确解析 Speedtest URL");
            return -1.0;
        },
    };

    // 解析 Domain
    let domain = match url.domain() {
        Some(tmp) => tmp.to_string(),
        None => {
            error!("无法获取测速链接中的域名");
            return -1.0;
        },
    };

    // 解析 Port
    let port = url.port().unwrap_or(443);
    
    // 解析路径
    let path = url.path();

    let server_name: rustls::pki_types::ServerName<'_> = domain.clone().try_into().unwrap();

    let mut conn: rustls::ClientConnection = rustls::ClientConnection::new(Arc::new(config), server_name).unwrap();

    let mut sock = match TcpStream::connect(format!("{}:{}", ip, port)) {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法初始化 TcpStream: {}", e);
            return -1.0;
        },
    };

    let mut tls: rustls::Stream<rustls::ClientConnection, TcpStream> = rustls::Stream::new(&mut conn, &mut sock);
    
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, domain
    );

    let start_time = Instant::now();
    let mut buffer = [0; 1024];
    let mut data = 0;

    match tls.write_all(request.as_bytes()) {
        Ok(_) => {},
        Err(e) => {
            error!("无法发送请求: {}", e);
            return -1.0;
        },
    }

    loop {
        match tls.read(&mut buffer) {
            Ok(0) => break, 
            Ok(n) => {
                data += n;
                
                if start_time.elapsed().as_secs_f64() >= speed_time as f64 {
                    break;
                }
            },
            Err(_) => break,
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