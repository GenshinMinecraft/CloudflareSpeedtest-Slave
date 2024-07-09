# CloudflareSpeedtest-Slave

一个轻量级、高性能的 Cloudflare IP Speedtest 后端，采用 Rust 编写

## 简介

本后端适用于 Moohr 开发的 Cloudflare Speedtest 主端，用于分布式 Cloudflare 测速

为 Moohr 开发的 Cloudflare Speedtest 主端**官方钦定**后端，如果有需要自行编写后端的需求可参考该项目

该项目**不支持 Windows** 系统运行，以后也不会支持，Never!

在华为仓颉编程语言推出后，本项目将第一时间使用其开发！

## 使用

```
Cloudflare IP Speedtest Backend

Usage: CloudflareSpeedtest-Slave [OPTIONS]

Options:
  -s, --server <SERVER>      Frontend Server Address [default: 1.1.1.1:2333]
  -t, --token <TOKEN>        Token Setting [default: 1145141919810]
  -m, --max-mbps <MAX_MBPS>  Bandwidth (in Mbps) [default: 500]
      --debug                Enable Debug Log
      --install              Install For Systemd
  -h, --help                 Print help
  -V, --version              Print version
```

- `-s`/`--server`: 指定主端服务器，默认为该项目官方服务器，请自行更改
- `-t`/`--token`: 连接主端时的鉴权 Token，请自行更改
- `-m`/`--max-mbps`: 报告给主端的最大带宽，单位 Mbps
- `--debug`: 开启 Debug Log
- `--install`: 使用 Systemd 安装 CloudflareSpeedtest-Slave，仅限于使用 Systemd 的 Linux
- `-h`: 显示此帮助
- `-V`/`--version`: 显示版本

## 鸣谢

感谢所有开源工作者！

- Cloudflare: 赞美大爹！
- 通义灵码: 为本项目提供了详细的注释编写，所以该项目中几乎所有的注释都是它写的