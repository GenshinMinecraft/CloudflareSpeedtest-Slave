fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure() // 禁用服务器端代码生成
        .compile(&["proto/cfst_rpc.proto"], &["proto/"])?;
    Ok(())
}