# CloudflareSpeedtest-Slave

![LICENSE](https://img.shields.io/github/license/GenshinMinecraft/CloudflareSpeedtest-Slave.svg)
![GitHub top language](https://img.shields.io/github/languages/top/GenshinMinecraft/CloudflareSpeedtest-Slave)
![GitHub commit activity](https://img.shields.io/github/commit-activity/w/GenshinMinecraft/CloudflareSpeedtest-Slave)
![GitHub repo size](https://img.shields.io/github/repo-size/GenshinMinecraft/CloudflareSpeedtest-Slave)

一个轻量级、高性能的 Cloudflare IP Speedtest 后端, 采用 Rust 编写

## 简介

本后端适用于 Moohr 开发的 Cloudflare Speedtest 主端, 用于分布式 Cloudflare 测速

为 Moohr 开发的 Cloudflare Speedtest 主端**官方钦定**后端, 如果有需要自行编写后端的需求可参考该项目

## 使用

```
A tool, written in Rust, for testing the speed of Cloudflare IPs.

Usage: CloudflareSpeedtest-Slave [OPTIONS]

Options:
  -s, --server <SERVER>       Frontend Server Address [default: 47.238.130.86:2333]
  -t, --token <TOKEN>         Token Setting [default: cfst1234]
  -m, --max-mbps <MAX_MBPS>   Bandwidth (in Mbps) [default: 500]
      --debug                 Enable Debug Log
      --install               Install For Systemd
      --disable-auto-upgrade  Disable Auto Upgrade ModeD
  -h, --help                  Print help
  -V, --version               Print version
```

- `-s`/`--server`: 指定主端服务器, 默认为该项目官方服务器, 请自行更改
- `-t`/`--token`: 连接主端时的鉴权 Token, 请自行更改
- `-m`/`--max-mbps`: 报告给主端的最大带宽, 单位 Mbps
- `--debug`: 开启 Debug Log
- `--install`: 使用 Systemd 安装 CloudflareSpeedtest-Slave, 仅限于使用 Systemd 的 Linux
- `--disable-auto-upgrade`: 禁用自动升级, 默认为开启
- `-h`: 显示此帮助
- `-V`/`--version`: 显示版本

## Docker 使用

首先, 请安装 Docker: 

```bash
curl -fsSL https://test.docker.com -o test-docker.sh
sudo sh test-docker.sh

# 如果您在中国大陆, 可能需要 Docker 镜像: 
curl -fsSL https://gdk.rtc.ovh | bash -s docker --mirror Aliyun
```

随后运行 Docker:

```bash
docker run -d --restart=always --name CloudflareSpeedtest-Slave \
-e TOKEN=cfst1234 \
-e MAX_MBPS=500 \
-e SERVER=backend.cloudflare.su:2333 \
dp.rtc.ovh/genshinminecraft/cloudflarespeedtest-slave:v0.0.6
```

目前, 我们只提供了 `arm64` / `amd64` 架构的镜像, 如果需要其他架构的镜像, 请自行编译主程序后编写 Dockerfile

## 贡献

我们欢迎 Issue 和 Pull Request, 也可以前往我们的[内测群组](https://t.me/+Gbqf_XAhVIphZmY1)反馈

## 编译

### Cargo

```bash
git clone https://github.com/GenshinMinecraft/CloudflareSpeedtest-Slave.git
cd CloudflareSpeedtest-Slave
cargo build --release --target x86_64-unknown-linux-musl # Or aarch64-unknown-linux-musl
./target/x86_64-unknown-linux-musl/release/CloudflareSpeedtest-Slave # Or aarch64-unknown-linux-musl
```

请注意: 当前我们尚未测试除了 `linux-x86_64` 与 `linux-arm64` 还有 `windows-x86_64` 的其他平台, 当您有其他系统 / 架构的需求, 请自行编译

### Docker

请事先将已经编译好的 `arm64` / `amd64` 二进制文件放入本项目根目录下的 `binary/` 文件夹 (没有请自行创建)

需要: `binary/arm64` 与 `binary/amd64`

```bash
docker buildx build --platform linux/amd64,linux/arm64 .
```

## 鸣谢

感谢所有开源工作者！

- Cloudflare: 赞美大爹！
- 通义灵码: 为本项目提供了详细的注释编写, 所以该项目中几乎所有的注释都是它写的