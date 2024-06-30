#!/bin/bash

# 设置循环次数
ITERATIONS=10090000000000000

# 循环执行grpcurl命令
for ((i=1; i<=$ITERATIONS; i++))
do
    echo "Executing grpcurl command iteration $i..."
    grpcurl -plaintext -import-path . -proto cfst_rpc.proto -d '{}' localhost:11451 cfst_rpc.CloudflareSpeedtest/Alive
    # 检查上一条命令的退出状态，非0则终止循环
    if [ $? -ne 0 ]; then
        echo "grpcurl command failed, exiting loop."
        break
    fi
done

echo "All iterations completed."

