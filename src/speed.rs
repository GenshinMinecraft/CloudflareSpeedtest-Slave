use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Instant,
};

use url::Url;
use native_tls::TlsConnector;

pub async fn speed_one_ip(speedtest_url: String, ip: String, speed_time: u32) -> i128 {

    let url = match Url::parse(speedtest_url.as_str()) {
        Ok(parsed_url) => parsed_url,
        Err(_) => {
            eprintln!("Failed to parse URL.");
            return -1;
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

    let time_seconds = start_time.elapsed().as_secs(); // 下载所花的时间，单位：秒，类型为 u64
    let megabytes_downloaded = data as f64 / 1_048_576.0;
    let time_seconds_f64 = time_seconds as f64;
    let download_speed_megabytes_per_second = megabytes_downloaded / time_seconds_f64;
    let download_speed_megabits_per_second = download_speed_megabytes_per_second * 8.0;

    download_speed_megabits_per_second.round() as i128
}