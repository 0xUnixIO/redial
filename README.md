# redial

通过 Telegram 机器人一键换 IP，专为拨号服务器设计。

## 功能

- `/status` — 查看所有服务器当前 IP 和今日剩余换 IP 次数
- `/change` — 触发换 IP（重拨），多台服务器时弹出选择菜单
- 换完自动验证连通性并回复新 IP

## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/zzfn/server/main/install.sh | bash
```

支持平台：Linux x86_64 / aarch64，macOS x86_64 / Apple Silicon

## 首次配置

安装后直接运行，自动进入配置向导：

```
$ redial

首次运行，开始配置向导...

Boil 账号（邮箱）: you@example.com
Boil 密码: ********

✅ 登录成功，找到以下服务器：

  服务器 A | IP: 1.2.3.xxx | 可换 IP ✅
  服务器 B | IP: 5.6.7.xxx | NAT 不可换

Telegram Bot Token: 123456:AAFxxx...
请向你的机器人发送任意消息，然后按回车继续...
✅ 检测到 chat_id: 392068270

✅ 配置已保存到 config.env
配置加载成功，启动 Telegram 机器人...
```

TG Bot 通过 [@BotFather](https://t.me/BotFather) 创建，发送 `/newbot` 获取 Token。

## 手动配置

也可以直接复制模板填写：

```bash
cp config.env.example config.env
# 编辑 config.env，填入账号密码和 TG 配置
redial
```

## 后台运行（Linux 服务器）

```bash
nohup redial >> bot.log 2>&1 &
```

用 systemd 常驻（推荐）：

```ini
# /etc/systemd/system/redial.service
[Unit]
Description=Boil IP Bot
After=network.target

[Service]
ExecStart=/usr/local/bin/redial
WorkingDirectory=/etc/redial
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo mkdir /etc/redial
sudo cp config.env /etc/redial/
sudo systemctl enable --now redial
```

## 从源码编译

```bash
git clone https://github.com/zzfn/server.git
cd server
cargo build --release
./target/release/redial
```
