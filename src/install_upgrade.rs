use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    process::{exit, Command},
};

use crate::{args::Args, cfst_rpc::*, cloudflare_speedtest_client::CloudflareSpeedtestClient};

use log::{error, info, warn};
use reqwest::Client;
use tonic::transport::Channel;

/// 安装并配置 Systemd 服务。
///
/// 此函数检查当前系统是否为 Linux, 并确认是否使用 Systemd 作为服务管理器。
/// 它还需要以 root 用户身份运行, 以复制可执行文件并修改系统服务配置。
/// 最后, 它将根据提供的参数配置并启动一个名为 cfst_slave.service 的 Systemd 服务。
pub fn install_systemd(args: Args) {
    // 检查操作系统是否为 Linux
    if env::consts::OS != "linux" {
        error!("Install 功能仅适用于 Linux 系统");
        exit(1);
    }

    // 检查系统是否使用 Systemd
    match fs::metadata("/usr/bin/systemctl") {
        Ok(_) => {
            info!("您的系统使用的是 Systemd 服务管理器, 可以正常使用 Install 功能")
        }
        Err(_) => {
            error!("您的系统并非使用 Systemd 作为服务管理器, 无法使用 Install 功能, 请自行配置进程守护");
            exit(1);
        }
    }

    // 确认以 root 用户身份运行
    if std::env::var("USER") == Ok("root".to_string()) {
        info!("正在使用 root 用户");
    } else {
        error!("非 root 用户, 请使用 root 用户运行 Install 功能");
        exit(1);
    }

    // 检查是否已存在相同名称的服务文件
    match fs::metadata("/etc/systemd/system/cfst_slave.service") {
        Ok(_) => loop {
            warn!("您的当前系统曾经配置过 Systemd 保活服务, 是否覆盖? (Y/N)");
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input = input.trim().to_uppercase();

            if input == "Y" {
                info!("正在为您覆盖先前的文件");
                break;
            } else if input == "N" {
                info!("不覆盖, 退出程序");
                exit(1);
            } else {
                warn!("输入错误, 请重新输入 Y 或 N。");
            }
        },
        Err(_) => {}
    }

    // 复制可执行文件到 /usr/bin
    match env::current_exe() {
        Ok(path_to_bin) => {
            if path_to_bin.to_str().unwrap() == "/usr/bin/CloudflareSpeedtest-Slave" {
                info!("无需复制可执行文件");
            } else {
                match Command::new("cp")
                    .arg("-afr")
                    .arg(path_to_bin)
                    .arg("/usr/bin/CloudflareSpeedtest-Slave")
                    .output()
                {
                    Ok(tmp) => {
                        if tmp.status.success() {
                            info!("成功将可执行文件复制到 /usr/bin/CloudflareSpeedtest-Slave");
                        } else {
                            error!("无法将可执行文件复制到 /usr/bin/CloudflareSpeedtest-Slave");
                            exit(1);
                        }
                    }
                    Err(e) => {
                        error!(
                            "无法将可执行文件复制到 /usr/bin/CloudflareSpeedtest-Slave: {}",
                            e
                        );
                        exit(1);
                    }
                }
            }
        }
        Err(e) => {
            error!("无法获取可执行文件路径: {}", e);
            exit(1);
        }
    }

    // 配置服务文件的内容
    let service_config = r#"[Unit]
Description=Cloudflare Speedtest Slave
After=network.target

[Install]
WantedBy=multi-user.target

[Service]
Type=simple
ExecStart=/usr/bin/CloudflareSpeedtest-Slave -s SERVERURL -t TOKEN -m MAXMBPS
Restart=always
"#;

    // 根据参数替换服务文件中的占位符
    let mut replaced_service_config = service_config.replace("SERVERURL", args.server.as_str());
    replaced_service_config = replaced_service_config.replace("TOKEN", args.token.as_str());
    if args.debug {
        let tmp = args.max_mbps.to_string() + " --debug";
        replaced_service_config = replaced_service_config.replace("MAXMBPS", tmp.as_str());
    } else {
        replaced_service_config =
            replaced_service_config.replace("MAXMBPS", args.max_mbps.to_string().as_str());
    }

    // 删除旧的服务文件
    match Command::new("rm")
        .arg("-rf")
        .arg("/etc/systemd/system/cfst_slave.service")
        .output()
    {
        Ok(_) => {}
        Err(e) => {
            error!("无法删除 /etc/systemd/system/cfst_slave.service: {}", e);
            exit(1);
        }
    }

    // 创建并写入新的服务文件
    let mut service_file = match File::create("/etc/systemd/system/cfst_slave.service") {
        Ok(tmp) => tmp,
        Err(e) => {
            error!("无法新建 /etc/systemd/system/cfst_slave.service: {}", e);
            exit(1);
        }
    };

    match service_file.write_all(replaced_service_config.as_bytes()) {
        Ok(_) => {
            info!("成功写入 Systemd 配置文件")
        }
        Err(e) => {
            error!("无法写入 Systemd 配置文件: {}", e);
            exit(1);
        }
    }

    // 重新加载 Systemd 配置
    match Command::new("systemctl").arg("daemon-reload").output() {
        Ok(tmp) => {
            if tmp.status.success() {
                info!("成功运行 systemctl daemon-reload");
            } else {
                error!("无法运行 systemctl daemon-reload")
            }
        }
        Err(e) => {
            error!("无法运行 systemctl daemon-reload: {}", e);
            exit(1);
        }
    }

    // 启动服务
    match Command::new("systemctl")
        .arg("start")
        .arg("cfst_slave.service")
        .output()
    {
        Ok(tmp) => {
            if tmp.status.success() {
                info!("成功启动 Cloudflare Speedtest Slave");
            } else {
                error!("无法启动 Cloudflare Speedtest Slave")
            }
        }
        Err(e) => {
            error!("无法启动 Cloudflare Speedtest Slave: {}", e);
            exit(1);
        }
    }

    // 询问用户是否开启开机自启动
    loop {
        info!("是否打开开机自启? (Y/N)");
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input = input.trim().to_uppercase();

        if input == "Y" {
            match Command::new("systemctl")
                .arg("enable")
                .arg("cfst_slave.service")
                .output()
            {
                Ok(tmp) => {
                    if tmp.status.success() {
                        info!("成功打开开机自启");
                    } else {
                        error!("无法打开开机自启")
                    }
                }
                Err(e) => {
                    error!("无法打开开机自启: {}", e);
                    exit(1);
                }
            }
            break;
        } else if input == "N" {
            info!("不打开, 退出程序");
            exit(1);
        } else {
            warn!("输入错误, 请重新输入 Y 或 N。");
        }
    }
}

// 异步函数, 负责检查并执行云flare速度测试客户端的更新。
// 参数:
// - client: 云flare速度测试客户端实例, 使用channel进行通信。
// - args: 命令行参数, 包含是否禁用自动升级等信息。
// - bootstrapres: 启动时从服务器获取的响应, 包含是否需要升级的信息。
pub async fn upgrade_bin(
    mut client: CloudflareSpeedtestClient<Channel>,
    args: Args,
    bootstrapres: BootstrapResponse,
) {
    // 检查是否需要升级, 如果不需升级则直接返回。
    if !bootstrapres.should_upgrade {
        info!("该后端为最新版本, 无需更新");
        return;
    } else {
        info!("准备开始更新后端");
    }

    // 如果配置了禁用自动升级, 则即使需要升级也不执行更新。
    if args.disable_auto_upgrade {
        warn!("该后端版本需更新, 但由于配置了 Disable Auto Upgrade, 不予更新");
        return;
    }

    info!("开始更新后端");

    // 尝试从客户端获取更新信息。
    let upgrade_message = match client.upgrade(UpgradeRequest {}).await {
        Ok(tmp) => {
            let result = tmp.into_inner();
            // 如果更新请求成功, 检查是否成功获取了更新链接。
            if result.success {
                info!("成功获取更新链接: {}", result.message);
                result
            } else {
                error!("无法获取更新链接, 终止更新继续运行: {}", result.message);
                return;
            }
        }
        Err(e) => {
            error!("无法获取更新链接, 终止更新继续运行: {}", e);
            return;
        }
    };

    // 根据更新信息下载对应的操作系统和架构的更新文件。
    let version_bin = match Client::new()
        .get(format!(
            "{}-{}-{}",
            upgrade_message.upgrade_url,
            env::consts::OS,
            env::consts::ARCH
        ))
        .send()
        .await
    {
        Ok(tmp) => {
            // 检查下载是否成功。
            if tmp.status().is_success() {
                tmp
            } else {
                error!(
                    "无法下载文件 URL: {}, Code: {}, 终止更新继续运行",
                    tmp.url().to_string(),
                    tmp.status().to_string()
                );
                return;
            }
        }
        Err(e) => {
            error!(
                "无法下载文件 URL: {}, 终止更新并继续运行: {}",
                format!(
                    "{}-{}-{}",
                    upgrade_message.upgrade_url,
                    env::consts::OS,
                    env::consts::ARCH
                ),
                e
            );
            return;
        }
    };

    // 在临时目录创建一个文件用于存储下载的更新二进制文件。
    let tmp_dir = env::temp_dir();
    let file_path = tmp_dir.join("CloudflareSpeedtest-Slave");

    match File::create(&file_path) {
        Ok(mut tmp) => {
            // 下载二进制文件内容。
            let binary = match version_bin.bytes().await {
                Ok(tmp) => tmp,
                Err(e) => {
                    error!("无法获取 Binary, 终止更新并继续运行: {}", e);
                    return;
                }
            };
            // 将二进制文件内容写入到临时文件。
            match tmp.write_all(&binary) {
                Ok(_) => {
                    info!("成功将 Binary 保存到 Temp Dir");
                }
                Err(e) => {
                    error!("无法将 Binary 保存到 Temp Dir, 终止更新并继续运行: {}", e);
                    return;
                }
            }
        }
        Err(e) => {
            error!("无法将 Binary 保存到 Temp Dir, 终止更新并继续运行: {}", e);
            return;
        }
    }

    // 为临时文件添加可执行权限。
    match Command::new("chmod")
        .arg("+x")
        .arg(file_path.clone())
        .output()
    {
        Ok(_) => {
            info!("成功添加可执行权限");
        }
        Err(e) => {
            error!("无法添加可执行权限, 终止更新并继续运行: {}", e);
            return;
        }
    }

    // 复制临时文件到当前执行程序的路径, 以替换旧版本。
    match env::current_exe() {
        Ok(path_to_bin) => {
            match Command::new("cp")
                .arg("-afr")
                .arg(file_path)
                .arg(path_to_bin)
                .output()
            {
                Ok(tmp) => {
                    // 检查复制是否成功。
                    if tmp.status.success() {
                        info!("成功将可执行文件替换");
                    } else {
                        error!("无法将可执行文件替换, 终止更新并继续运行");
                        return;
                    }
                }
                Err(e) => {
                    error!("无法将可执行文件替换, 终止更新并继续运行: {}", e);
                    return;
                }
            }
        }
        Err(e) => {
            error!("无法获取当前运行程序路径, 终止更新并继续运行: {}", e);
            return;
        }
    }

    // 启动新的可执行文件, 替换当前进程。
    let mut command = Command::new(env::current_exe().unwrap());
    command.args(env::args().skip(1));

    let _ = match command.spawn() {
        Ok(_) => {
            info!("成功启动新程序");
            exit(1);
        }
        Err(e) => {
            error!(
                "无法启动新程序, 主程序将退出, 请自行重新启动新版本程序: {}",
                e
            );
            exit(1);
        }
    };
}
