# boil

为拨号服务器设计的换 IP 工具，支持命令行直接操作和 Telegram 机器人远程控制。

## 功能

- `status` — 查看所有服务器当前 IP 和今日剩余次数
- `check` — 检查当前 IP 质量（类型/ISP/CF 风险/流媒体解锁）
- `change` — 换 IP（重拨），多台服务器时交互选择
- `timer` — 设置 cron 定时自动换 IP
- Telegram Bot 可选，支持远程控制和换 IP 结果推送

## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/0xUnixIO/boil/main/install.sh | bash
```

支持平台：Linux x86_64 / aarch64

安装完成后运行配置向导：

```bash
boil setup
```

Telegram Bot 通过 [@BotFather](https://t.me/BotFather) 创建，发送 `/newbot` 获取 Token。

## 命令

```bash
boil                         # 交互菜单（未配置 TG）或启动机器人（已配置 TG）
boil status                  # 查看当前 IP 和今日剩余次数
boil check                   # 检查 IP 质量和流媒体解锁
boil change                  # 换 IP
boil timer                   # 查看定时设置
boil timer "0 */6 * * *"     # 设置定时：每6小时
boil timer "0 3 * * *"       # 设置定时：每天凌晨3点
boil timer off               # 关闭定时
boil bot                     # 启动 Telegram 机器人
boil setup                   # 重新运行配置向导
```

## Telegram 命令

| 命令 | 说明 |
|------|------|
| `/status` | 查看当前 IP 和今日剩余次数 |
| `/check` | 检查 IP 质量和流媒体解锁 |
| `/change` | 换 IP，多台时弹出选择 |
| `/timer` | 查看定时设置 |
| `/timer 0 */6 * * *` | 设置定时（cron 5字段） |
| `/timer off` | 关闭定时 |

## 从源码编译

```bash
git clone https://github.com/0xUnixIO/boil.git
cd boil
cargo build --release
./target/release/boil
```
