#!/usr/bin/env bash
#
# Boil.network 换 IP — Telegram 交互机器人
# 支持命令: /status  /change
#
# 依赖: curl, jq
# 启动: bash tg-bot.sh
# 后台: nohup bash tg-bot.sh >> bot.log 2>&1 &

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${CONFIG_FILE:-$SCRIPT_DIR/config.env}"
[ -f "$CONFIG_FILE" ] && source "$CONFIG_FILE"

: "${BOIL_ACCOUNT:?请配置 BOIL_ACCOUNT}"
: "${BOIL_PASSWORD:?请配置 BOIL_PASSWORD}"
: "${BOIL_ROUTER_ID:=182}"
: "${BOIL_INTERFACE:=adsl3}"
: "${TG_TOKEN:?请配置 TG_TOKEN}"
: "${TG_CHAT_ID:?请配置 TG_CHAT_ID}"

BASE_URL="https://ippanel.boil.network"
TG_API="https://api.telegram.org/bot${TG_TOKEN}"

RECONNECT_WAIT="${RECONNECT_WAIT:-8}"
POLL_TIMES="${POLL_TIMES:-10}"
POLL_INTERVAL="${POLL_INTERVAL:-3}"

# ---------- TG 工具 ----------
tg_send() {
  curl -fsS "$TG_API/sendMessage" \
    -d "chat_id=$TG_CHAT_ID" \
    --data-urlencode "text=$1" \
    --data-urlencode "parse_mode=Markdown" \
    -o /dev/null
}

tg_updates() {
  # $1 = offset
  curl -fsS "$TG_API/getUpdates?offset=$1&timeout=30&limit=10"
}

log() { printf '[%s] %s\n' "$(date '+%H:%M:%S')" "$*"; }

# ---------- Boil 登录 ----------
COOKIE_JAR=""

boil_login() {
  [ -n "$COOKIE_JAR" ] && rm -f "$COOKIE_JAR"
  COOKIE_JAR="$(mktemp -t boil-cookie.XXXXXX)"
  curl -fsS -c "$COOKIE_JAR" \
    -X POST "$BASE_URL/login" \
    -H 'content-type: application/x-www-form-urlencoded' \
    --data-urlencode "account=$BOIL_ACCOUNT" \
    --data-urlencode "password=$BOIL_PASSWORD" \
    -o /dev/null
  grep -q 'session' "$COOKIE_JAR" || return 1
}

api_post() {
  curl -fsS -b "$COOKIE_JAR" \
    -X POST "$BASE_URL$1" \
    -H 'content-type: application/json' \
    --data-raw "$2"
}

extract_ip() {
  jq -r --arg rid "$BOIL_ROUTER_ID" --arg ifc "$BOIL_INTERFACE" \
    '.results[$rid][$ifc] // empty' 2>/dev/null
}

# ---------- 子命令实现 ----------
do_status() {
  boil_login || { tg_send "❌ 登录失败，请检查账号密码"; return; }
  local all cur used limit
  all="$(api_post /api/query_all '{}')"
  cur="$(echo "$all" | extract_ip)"
  used="$(echo "$all" | jq -r '.daily_used // "?"')"
  limit="$(echo "$all" | jq -r '.daily_limit // "?"')"
  tg_send "📡 *当前状态*
IP: \`${cur:-未知}\`
今日换 IP: ${used}/${limit} 次"
  rm -f "$COOKIE_JAR"; COOKIE_JAR=""
}

do_change() {
  tg_send "⏳ 开始换 IP，请稍候..."
  boil_login || { tg_send "❌ 登录失败，请检查账号密码"; return; }

  local all old_ip used limit
  all="$(api_post /api/query_all '{}')"
  old_ip="$(echo "$all" | extract_ip || true)"
  used="$(echo "$all" | jq -r '.daily_used // 0')"
  limit="$(echo "$all" | jq -r '.daily_limit // 0')"

  if [ "$limit" -gt 0 ] && [ "$used" -ge "$limit" ]; then
    tg_send "⛔ 今日换 IP 额度已用尽（${used}/${limit}），明日再试"
    rm -f "$COOKIE_JAR"; COOKIE_JAR=""; return
  fi

  # 触发重拨
  api_post /api/reconnect \
    "{\"router_id\":\"$BOIL_ROUTER_ID\",\"interface\":\"$BOIL_INTERFACE\"}" \
    -o /dev/null 2>/dev/null || true

  sleep "$RECONNECT_WAIT"

  # 轮询新 IP
  local new_ip i
  for ((i=1; i<=POLL_TIMES; i++)); do
    new_ip="$(api_post /api/query_all '{}' | extract_ip || true)"
    if [ -n "$new_ip" ] && [ "$new_ip" != "$old_ip" ]; then
      break
    fi
    sleep "$POLL_INTERVAL"
  done

  rm -f "$COOKIE_JAR"; COOKIE_JAR=""

  if [ -z "${new_ip:-}" ] || [ "$new_ip" = "$old_ip" ]; then
    tg_send "⚠️ 重拨已触发，但未检测到 IP 变化（旧 IP: \`${old_ip:-未知}\`），请到面板确认"
    return
  fi

  # ping 验证
  local ping_status="（ping 不通，可能禁 ICMP，请自行确认）"
  ping -c 2 -t 5 "$new_ip" >/dev/null 2>&1 && ping_status="（ping 通 ✅）"

  tg_send "✅ *换 IP 完成*
旧 IP: \`${old_ip:-未知}\`
新 IP: \`${new_ip}\` ${ping_status}
剩余次数: $((limit - used - 1))/${limit}"
}

# ---------- 长轮询主循环 ----------
cleanup() {
  [ -n "${COOKIE_JAR:-}" ] && rm -f "$COOKIE_JAR"
  log "机器人已停止"
}
trap cleanup EXIT INT TERM

log "机器人启动，监听命令中..."
tg_send "🤖 Boil IP Bot 已上线，支持命令：/status /change"

offset=0
while true; do
  updates="$(tg_updates "$offset" 2>/dev/null || true)"
  [ -z "$updates" ] && continue

  count="$(echo "$updates" | jq '.result | length' 2>/dev/null || echo 0)"
  [ "$count" -eq 0 ] && continue

  for ((i=0; i<count; i++)); do
    row="$(echo "$updates" | jq -r ".result[$i]")"
    update_id="$(echo "$row" | jq -r '.update_id')"
    offset=$((update_id + 1))

    # 只处理来自授权 chat 的消息
    chat_id="$(echo "$row" | jq -r '.message.chat.id // empty')"
    [ "$chat_id" != "$TG_CHAT_ID" ] && continue

    cmd="$(echo "$row" | jq -r '.message.text // empty' | awk '{print $1}')"
    log "收到命令: $cmd"

    case "$cmd" in
      /status)  do_status ;;
      /change)  do_change ;;
      /start)   tg_send "👋 你好！命令列表：
/status — 查看当前 IP 和今日剩余次数
/change — 触发换 IP（重拨）" ;;
      *)  tg_send "❓ 未知命令。支持：/status /change" ;;
    esac
  done
done
