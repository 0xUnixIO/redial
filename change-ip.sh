#!/usr/bin/env bash
#
# Boil.network 拨号服务器换 IP 脚本
#
# 子命令:
#   status   只读:登录并打印当前服务器/IP 列表(不会换 IP),用于排查
#   change   登录 → 触发重拨换 IP → 等待 → 读取并验证新 IP → 通知(默认)
#
# 凭证从同目录 config.env 读取(见 config.env.example),也可用环境变量覆盖。

set -euo pipefail

# ---------- 配置加载 ----------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${CONFIG_FILE:-$SCRIPT_DIR/config.env}"
if [ -f "$CONFIG_FILE" ]; then
  # shellcheck disable=SC1090
  source "$CONFIG_FILE"
fi

: "${BOIL_ACCOUNT:?请在 config.env 配置 BOIL_ACCOUNT}"
: "${BOIL_PASSWORD:?请在 config.env 配置 BOIL_PASSWORD}"
: "${BOIL_ROUTER_ID:=182}"
: "${BOIL_INTERFACE:=adsl3}"

BASE_URL="https://ippanel.boil.network"

# 换 IP 后等待重拨完成的秒数 / 轮询次数
RECONNECT_WAIT="${RECONNECT_WAIT:-8}"
POLL_TIMES="${POLL_TIMES:-10}"
POLL_INTERVAL="${POLL_INTERVAL:-3}"

# ---------- 临时 cookie 罐 ----------
COOKIE_JAR="$(mktemp -t boil-cookie.XXXXXX)"
trap 'rm -f "$COOKIE_JAR"' EXIT

log()  { printf '[%s] %s\n' "$(date '+%H:%M:%S')" "$*" >&2; }
die()  { log "错误: $*"; exit 1; }

# ---------- 通知占位(以后接 Telegram/邮件等往这里填) ----------
notify() {
  # $1 = 标题, $2 = 正文
  # 例如以后接 Telegram:
  #   curl -s "https://api.telegram.org/bot$TG_TOKEN/sendMessage" \
  #        -d "chat_id=$TG_CHAT" --data-urlencode "text=$1"$'\n'"$2" >/dev/null
  log "通知(未配置渠道): $1 - $2"
}

# ---------- 登录,拿到 session cookie ----------
login() {
  log "登录中: $BOIL_ACCOUNT"
  curl -fsS -c "$COOKIE_JAR" \
    -X POST "$BASE_URL/login" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "account=$BOIL_ACCOUNT" \
    --data-urlencode "password=$BOIL_PASSWORD" \
    -o /dev/null || die "登录请求失败"

  # 校验是否真的拿到了登录态(query_all 能正常返回即视为成功)
  grep -q 'session' "$COOKIE_JAR" || die "登录后未获得 session cookie,检查账号密码"
  log "登录成功"
}

# ---------- 已登录态下调用 JSON API ----------
api_post() {
  # $1 = 路径(如 /api/query_all), $2 = JSON body
  curl -fsS -b "$COOKIE_JAR" \
    -X POST "$BASE_URL$1" \
    -H 'content-type: application/json' \
    --data-raw "$2"
}

# ---------- 子命令:只读查看状态 ----------
cmd_status() {
  login
  log "拉取 query_all ..."
  local all
  all="$(api_post /api/query_all '{}')"
  echo "$all" | jq .
  # 摘要
  local cur used limit
  cur="$(echo "$all" | extract_ip)"
  used="$(echo "$all" | jq -r '.daily_used // "?"')"
  limit="$(echo "$all" | jq -r '.daily_limit // "?"')"
  log "目标 $BOIL_ROUTER_ID/$BOIL_INTERFACE 当前 IP: ${cur:-未知} | 今日已换 $used/$limit 次"
}

# ---------- 触发换 IP(重拨) ----------
reconnect() {
  log "触发重拨: router_id=$BOIL_ROUTER_ID interface=$BOIL_INTERFACE"
  api_post /api/reconnect \
    "{\"router_id\":\"$BOIL_ROUTER_ID\",\"interface\":\"$BOIL_INTERFACE\"}"
}

# ---------- 从 query_all 返回里提取当前 router 的 IP ----------
# 结构:.results.<router_id>.<interface> = "公网IP"
extract_ip() {
  # stdin = query_all 的 JSON
  jq -r --arg rid "$BOIL_ROUTER_ID" --arg ifc "$BOIL_INTERFACE" \
    '.results[$rid][$ifc] // empty' 2>/dev/null
}

# ---------- 子命令:换 IP ----------
cmd_change() {
  login

  local all old_ip new_ip resp used limit
  all="$(api_post /api/query_all '{}')"
  old_ip="$(echo "$all" | extract_ip || true)"
  used="$(echo "$all" | jq -r '.daily_used // 0')"
  limit="$(echo "$all" | jq -r '.daily_limit // 0')"
  log "换前 IP: ${old_ip:-未知} | 今日已换 $used/$limit 次"

  if [ "$limit" -gt 0 ] && [ "$used" -ge "$limit" ]; then
    notify "换 IP 跳过" "今日额度已用尽($used/$limit)"
    die "今日换 IP 额度已用尽($used/$limit),明日再试"
  fi

  resp="$(reconnect)"
  log "reconnect 返回: $resp"

  log "等待重拨完成(${RECONNECT_WAIT}s)..."
  sleep "$RECONNECT_WAIT"

  # 轮询直到 IP 变化或超时
  local i
  for ((i=1; i<=POLL_TIMES; i++)); do
    new_ip="$(api_post /api/query_all '{}' | extract_ip || true)"
    if [ -n "$new_ip" ] && [ "$new_ip" != "$old_ip" ]; then
      break
    fi
    log "第 $i 次查询: 当前 ${new_ip:-未知},等待 IP 变化..."
    sleep "$POLL_INTERVAL"
  done

  if [ -z "${new_ip:-}" ]; then
    notify "换 IP 异常" "重拨已触发,但未能读取到新 IP,请到面板确认"
    die "未能读取到新 IP"
  fi

  log "新 IP: $new_ip"

  # 验证连通性
  if ping -c 2 -t 5 "$new_ip" >/dev/null 2>&1; then
    log "连通性验证: 通 (ping $new_ip 成功)"
    notify "换 IP 成功" "新 IP: $new_ip (已 ping 通,旧 IP: ${old_ip:-未知})"
  else
    log "连通性验证: ping 不通(部分服务器禁 ICMP,未必代表故障)"
    notify "换 IP 完成" "新 IP: $new_ip (ping 不通,请自行确认,旧 IP: ${old_ip:-未知})"
  fi

  echo "$new_ip"
}

# ---------- 入口 ----------
main() {
  case "${1:-}" in
    status) cmd_status ;;
    change) cmd_change ;;
    *) echo "用法: $0 {status|change}" >&2; exit 2 ;;
  esac
}

main "$@"
