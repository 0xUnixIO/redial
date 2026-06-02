# boil

为拨号服务器设计的换 IP 工具，支持命令行直接操作和 Telegram 机器人远程控制。

## 功能

- `status` — 查看所有服务器当前 IP 和今日剩余次数
- `check` — 检查当前 IP 质量（类型/ISP/CF 风险）
- `change` — 换 IP（重拨），多台服务器时交互选择
- `timer` — 设置 cron 定时自动换 IP
- Telegram Bot 可选，支持远程控制和换 IP 结果推送

## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/0xUnixIO/boil/main/install.sh | bash
```

支持平台：Linux x86_64 / aarch64

## 首次配置

安装后直接运行，自动进入配置向导：

```
$ boil

未找到配置，启动首次配置向导...

Boil 账号（邮箱）: you@example.com
Boil 密码: ********

✅ 登录成功，找到以下服务器：

  服务器 A | IP: 1.2.3.xxx | 可换 IP ✅
  服务器 B | IP: 5.6.7.xxx | NAT 不可换

配置 Telegram Bot（用于远程控制，可选）[Y/n]: n
已跳过 Telegram 配置，可使用 boil status/change 命令行操作

✅ 配置已保存到 config.env
```

Telegram Bot 通过 [@BotFather](https://t.me/BotFather) 创建，发送 `/newbot` 获取 Token。

## 命令

```bash
boil                    # 有 TG 配置则启动机器人，否则显示帮助
boil status           # 查看当前 IP 和今日剩余次数
boil check            # 检查当前 IP 质量
boil change           # 换 IP
boil timer            # 查看定时设置
boil timer "0 */6 * * *"   # 设置定时：每6小时
boil timer "0 3 * * *"     # 设置定时：每天凌晨3点
boil timer off        # 关闭定时
boil bot              # 启动 Telegram 机器人
boil setup            # 重新运行配置向导
boil service install  # 安装 systemd 服务（开机自启）
boil service uninstall # 卸载服务
```

## Telegram 命令

| 命令 | 说明 |
|------|------|
| `/status` | 查看当前 IP 和今日剩余次数 |
| `/check` | 检查当前 IP 质量 |
| `/change` | 换 IP，多台时弹出选择 |
| `/timer` | 查看定时设置 |
| `/timer 0 */6 * * *` | 设置定时（cron 5字段） |
| `/timer off` | 关闭定时 |

## 常驻运行

**systemd（推荐）：**

```bash
boil service install
```

安装后开机自启，崩溃自动重启。常用命令：

```bash
systemctl status  boil
systemctl restart boil
journalctl -fu    boil
```

**后台运行：**

```bash
nohup boil >> bot.log 2>&1 &
```

## 从源码编译

```bash
git clone https://github.com/0xUnixIO/boil.git
cd boil
cargo build --release
./target/release/redial
```
