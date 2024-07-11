use clap::Parser;

/// Cloudflare IP Speedtest Backend
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    // 主端地址
    /// Frontend Server Address
    #[arg(short, long, default_value_t = return_default_server())]
    pub server: String,

    // Bootstrap Token 设置
    /// Token Setting
    #[arg(short, long, default_value_t = return_default_bootstrap_token())]
    pub token: String,

    // 最大带宽
    /// Bandwidth (in Mbps)
    #[arg(short, long, default_value_t = 500)]
    pub max_mbps: i32,

    // Debug Log 设置
    /// Enable Debug Log
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    // 开始 Install
    /// Install For Systemd
    #[arg(long, default_value_t = false)]
    pub install: bool,

    // 关闭自动更新
    /// Disable Auto Upgrade Mode
    #[arg(long, default_value_t = false)]
    pub disable_auto_upgrade: bool,
}

/**
 * 返回默认服务器的地址。
 *
 * 该函数生成并返回一个字符串, 包含了默认服务器的IP地址和端口号。
 * 这是一个硬编码的值, 用于在没有特定服务器配置的情况下提供一个默认的选择。
 *
 * @return String 返回一个字符串, 格式为"IP地址:端口号"。
 */
fn return_default_server() -> String {
    return "backend.cloudflare.su:2333".to_string();
}

/**
 * 返回默认的启动令牌字符串。
 *
 * 此函数生成一个固定的字符串作为默认的启动令牌。这个令牌用于应用程序启动时的特定验证或配置过程。
 * 选择“cfst1234”作为默认值是因为它是一个预先定义的、不会引起混淆的值。
 *
 * @return 字符串类型的默认启动令牌。
 */
fn return_default_bootstrap_token() -> String {
    return "cfst1234".to_string();
}

/**
 * 初始化程序的参数对象。
 *
 * 该函数通过解析命令行参数, 创建并返回一个Args对象。
 * Args对象包含了程序运行时的所有配置参数, 这些参数可以通过命令行进行定制。
 * 
 * 返回值:
 * Args - 一个包含了程序运行参数的数据结构。
 */
pub fn init_args() -> Args {
    // 使用Args::parse方法从命令行参数中构建Args对象。
    let args: Args = Args::parse();
    // 返回构建好的Args对象。
    return args;
}