use log::{error, info};
use std::net::ToSocketAddrs;
use std::process::exit;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_rustls::{rustls, TlsConnector};
use url::Url;

/**
 * 测量给定IP地址和URL的下载速度。
 *
 * @param speedtest_url 测速URL, 用于发起下载请求。
 * @param ip 要测试速度的IP地址。
 * @param speed_time 测速时间（秒）, 用于限制下载时间。
 * @return 返回下载速度（Mbps）。
 */
pub async fn speed_one_ip(speedtest_url: String, ip: String, speed_time: u32) -> f64 {
    let url = match Url::parse(speedtest_url.as_str()) {
        Ok(parsed_url) => parsed_url,
        Err(e) => {
            error!("无法正确解析 Speedtest URL: {}", e);
            return -1.0;
        }
    };

    let domain = match &url.domain() {
        Some(tmp) => match rustls::pki_types::ServerName::try_from(tmp.to_string()) {
            Ok(tmp) => tmp,
            Err(e) => {
                error!("无法获取 Speedtest URL 中的域名: {}", e);
                return -1.0;
            }
        },
        None => {
            error!("无法获取 Speedtest URL 中的域名");
            return -1.0;
        }
    };

    let port = url.port().unwrap_or(443);

    let addr = match (ip.as_str(), port).to_socket_addrs() {
        Ok(mut iter) => match iter.next() {
            Some(addr) => addr,
            None => {
                error!("无法正确解析 Speedtest URL");
                return -1.0;
            }
        },
        Err(e) => {
            error!("无法正确解析 Speedtest URL: {}", e);
            return -1.0;
        }
    };

    let path = url.path();

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path,
        domain.to_str()
    );

    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let stream = match TcpStream::connect(&addr).await {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法创立 Tcp 连接: {}", e);
            return -1.0;
        }
    };

    let mut stream = match connector.connect(domain, stream).await {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法创立 Tls 连接: {}", e);
            return -1.0;
        }
    };

    match stream.write_all(request.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
            error!("无法写入请求: {}", e)
        }
    }

    let start_time = Instant::now();

    let mut buffer = [0; 1024];

    let mut data = 0;

    loop {
        match stream.read(&mut buffer).await {
            // 读取结束, 退出循环。
            // 没有则退出
            Ok(0) => break,
            // 成功读取数据, 累加到总下载大小。
            // 有则把接收到的放到计数器里
            Ok(n) => {
                data += n;
                if start_time.elapsed().as_secs_f64() >= speed_time as f64 {
                    break;
                }
            }
            Err(e) => {
                eprintln!("下载文件出现错误: {}", e);
                exit(1);
            }
        }
    }

    // 计算下载速度（Mbps）。
    // 已下载
    let bytes_downloaded = data;

    // 总用时
    let time_taken: f64 = start_time.elapsed().as_secs_f64();

    // 下载总 Bits
    let bits_downloaded: f64 = bytes_downloaded as f64 * 8.0;

    // bps 计算
    let download_speed_bps: f64 = bits_downloaded / time_taken;

    // bps -> kbps
    let download_speed_kbps: f64 = download_speed_bps / 1000.0;

    // kbps -> mbps
    let download_speed_mbps: f64 = download_speed_kbps / 1000.0;

    // 记录测速结果。
    info!("IP: {}, 速度: {}mbps", addr.ip(), download_speed_mbps);

    download_speed_mbps
}
