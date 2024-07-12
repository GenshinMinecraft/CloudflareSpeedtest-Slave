FROM alpine:latest

ARG TARGETARCH

ENV SERVER=backend.cloudflare.su:2333
ENV TOKEN=cfst1234
ENV MAX_MBPS=500

## 你必须要先将二进制文件保存在 ./binray/linux/{amd|arm}64 目录下
COPY ./binray/$TARGETARCH /CloudflareSpeedtest-Slave

CMD /CloudflareSpeedtest-Slave -s "$SERVER" -t "$TOKEN" -m "${MAX_MBPS}" --debug --disable-up
