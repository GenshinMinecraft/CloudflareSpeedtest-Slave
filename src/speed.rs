use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Instant,
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

    let domain = url.domain().unwrap();
    let port = url.port().unwrap_or(443);
    let path = url.path();

    let stream = TcpStream::connect(format!("{}:{}", ip, port)).unwrap();
    let connector = TlsConnector::builder().build().unwrap();
    let mut stream = connector.connect(domain, stream).unwrap();
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, domain
    );

    let start_time = Instant::now();
    let mut buffer = [0; 10240];
    let mut data = 0;

    stream.write_all(request.as_bytes()).unwrap();

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

    let bytes_downloaded = data; // 假设下载了 1MB 的数据
    let time_taken: f64 = start_time.elapsed().as_secs_f64();


    let bits_downloaded: f64 = bytes_downloaded as f64 * 8.0; // 将字节转换为位
    let download_speed_bps: f64 = bits_downloaded / time_taken; // 计算位每秒速度

    // 将速度转换为其他单位
    let download_speed_kbps: f64 = download_speed_bps / 1000.0;
    let download_speed_mbps: f64 = download_speed_kbps / 1000.0;

    info!("IP: {}, 速度: {}mbps", ip, download_speed_mbps);

    download_speed_mbps
}