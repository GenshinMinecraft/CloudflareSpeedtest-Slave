/**
 * 返回默认服务器的地址。
 *
 * 该函数生成并返回一个字符串，包含了默认服务器的IP地址和端口号。
 * 这是一个硬编码的值，用于在没有特定服务器配置的情况下提供一个默认的选择。
 *
 * @return String 返回一个字符串，格式为"IP地址:端口号"。
 */
pub fn return_default_server() -> String {
    return "47.238.130.86:2333".to_string();
}

/**
 * 返回默认的启动令牌字符串。
 *
 * 此函数生成一个固定的字符串作为默认的启动令牌。这个令牌用于应用程序启动时的特定验证或配置过程。
 * 选择“cfst1234”作为默认值是因为它是一个预先定义的、不会引起混淆的值。
 *
 * @return 字符串类型的默认启动令牌。
 */
pub fn return_default_bootstrap_token() -> String {
    return "cfst1234".to_string();
}