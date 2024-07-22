use std::{
    io::{Read, Write},
    sync::Arc,
    net::TcpStream,
    time::Instant,
};
use std::net::Shutdown;
use log::{error, info};
use rustls::RootCertStore;
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
    info!("开始测速 IP: {}, URL: {}", ip, speedtest_url);

    // 设置CA证书存储, 用于TLS连接验证服务器证书。
    // 设置 CA 证书
    let root_store = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.into(),
    };

    // 构建TLS客户端配置, 包括根证书和不验证客户端证书。
    // 创建 Rustls Config
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // 解析测速URL, 以获取域名和端口等信息。
    // 解析 Url
    let url = match Url::parse(speedtest_url.as_str()) {
        Ok(parsed_url) => parsed_url,
        Err(_) => {
            error!("无法正确解析 Speedtest URL");
            return -1.0;
        }
    };

    // 提取域名, 用于TLS连接的服务器名称标识。
    // 解析 Domain
    let domain = match url.domain() {
        Some(tmp) => tmp.to_string(),
        None => {
            error!("无法获取测速链接中的域名");
            return -1.0;
        }
    };

    // 默认端口为443, 如果URL中未指定。
    // 解析 Port
    let port = url.port().unwrap_or(443);

    // 构建HTTP请求路径。
    // 解析路径
    let path = url.path();

    // 将域名转换为Rustls所需的服务器名称类型。
    // 解析主机名
    let server_name: rustls::pki_types::ServerName<'_> = match domain.clone().try_into() {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法初始化 ServerName: {}", e);
            return -1.0;
        }
    };

    // 初始化TLS客户端连接。
    // 建立 TLS 连接
    let mut conn: rustls::ClientConnection =
        match rustls::ClientConnection::new(Arc::new(config), server_name) {
            Ok(tmp) => tmp,
            Err(e) => {
                error!("无法建立正确的 TLS 连接: {}", e);
                return -1.0;
            }
        };

    // 建立TCP连接到指定的IP和端口。
    // 建立 Tcp 连接
    let mut sock = match TcpStream::connect(format!("{}:{}", ip, port)) {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法初始化 TcpStream: {}", e);
            return -1.0;
        }
    };

    // 将TCP连接封装为TLS连接。
    // 建立 Tls 连接
    let mut tls: rustls::Stream<rustls::ClientConnection, TcpStream> =
        rustls::Stream::new(&mut conn, &mut sock);

    // 构建HTTP GET请求。
    // 请求内容
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, domain
    );

    // 记录开始时间, 用于计算下载速度。
    // 计时
    let start_time = Instant::now();

    // 初始化缓冲区, 用于接收下载数据。
    // 缓冲区
    let mut buffer = [0; 1024];

    // 初始化下载数据总大小。
    // 文件的总大小
    let mut data = 0;

    // 发送HTTP请求。
    // 发送请求
    match tls.write_all(request.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            error!("无法发送请求: {}", e);
            return -1.0;
        }
    }

    // 循环读取TLS连接的数据, 直到读取结束或达到测速时间。
    // 持续读取请求
    loop {
        // 循环
        match tls.read(&mut buffer) {
            // 读取结束, 退出循环。
            // 没有则退出
            Ok(0) => break,
            // 成功读取数据, 累加到总下载大小。
            // 有则把接收到的放到计数器里
            Ok(n) => {
                data += n;

                // 检查是否达到测速时间。
                // 检测是否超时
                if start_time.elapsed().as_secs_f64() >= speed_time as f64 {
                    break;
                }
            }
            // 读取数据出错, 退出循环。
            Err(e) => {
                error!("下载文件出现错误: {}", e);
                return -1.0;
            }
        }
    }

    sock.shutdown(Shutdown::Both).unwrap();
    drop(sock);

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
    info!("IP: {}, 速度: {}mbps", ip, download_speed_mbps);

    return download_speed_mbps;
}
