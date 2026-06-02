use std::{sync::Arc, time::Duration};

use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode},
    utils::command::BotCommands,
};
use tokio::time::sleep;

use crate::{boil::BoilClient, config::Config};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "命令列表:")]
enum Command {
    #[command(description = "开始使用")]
    Start,
    #[command(description = "查看当前 IP 和今日剩余次数")]
    Status,
    #[command(description = "换 IP（重拨）")]
    Change,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let bot = Bot::new(&config.tg_token);
    let config = Arc::new(config);

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(
            Update::filter_callback_query()
                .endpoint(handle_callback),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    config: Arc<Config>,
) -> ResponseResult<()> {
    if msg.chat.id.to_string() != config.tg_chat_id {
        return Ok(());
    }
    match cmd {
        Command::Start => {
            bot.send_message(
                msg.chat.id,
                "👋 <b>Boil IP Bot</b>\n\n/status — 查看当前 IP 和今日剩余次数\n/change — 换 IP（重拨）",
            )
            .parse_mode(ParseMode::Html)
            .await?;
        }
        Command::Status => cmd_status(&bot, msg.chat.id, &config).await,
        Command::Change => cmd_change(&bot, msg.chat.id, &config).await,
    }
    Ok(())
}

async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    config: Arc<Config>,
) -> ResponseResult<()> {
    if q.from.id.to_string() != config.tg_chat_id {
        bot.answer_callback_query(&q.id).await?;
        return Ok(());
    }
    bot.answer_callback_query(&q.id).await?;

    let chat_id = match &q.message {
        Some(msg) => msg.chat.id,
        None => return Ok(()),
    };

    if let Some(data) = &q.data {
        if let Some(rest) = data.strip_prefix("change:") {
            let mut parts = rest.splitn(2, ':');
            if let (Some(router_id), Some(interface)) = (parts.next(), parts.next()) {
                do_reconnect(&bot, chat_id, &config, router_id, interface).await;
            }
        }
    }
    Ok(())
}

async fn cmd_status(bot: &Bot, chat_id: ChatId, config: &Config) {
    let result = async {
        let c = BoilClient::new()?;
        c.login(&config.boil_account, &config.boil_password).await?;
        c.query_all().await
    }
    .await;

    match result {
        Ok(data) => {
            let mut lines = vec![format!(
                "📡 <b>服务器状态</b> | 今日换 IP {}/{} 次\n",
                data.daily_used, data.daily_limit
            )];
            for item in &data.zone_items {
                let ip = data
                    .get_ip(&item.router_id, &item.interface)
                    .unwrap_or("未知");
                let tag = if item.nat_no_change { "🔒 NAT" } else { "✅ 可换" };
                lines.push(format!("{}\n<code>{}</code> | {}", item.label, ip, tag));
            }
            let _ = bot
                .send_message(chat_id, lines.join("\n"))
                .parse_mode(ParseMode::Html)
                .await;
        }
        Err(e) => {
            let _ = bot
                .send_message(chat_id, format!("❌ 查询失败: {e}"))
                .await;
        }
    }
}

async fn cmd_change(bot: &Bot, chat_id: ChatId, config: &Config) {
    let result = async {
        let c = BoilClient::new()?;
        c.login(&config.boil_account, &config.boil_password).await?;
        c.query_all().await
    }
    .await;

    let data = match result {
        Ok(d) => d,
        Err(e) => {
            let _ = bot
                .send_message(chat_id, format!("❌ 登录失败: {e}"))
                .await;
            return;
        }
    };

    let changeable = data.changeable();
    if changeable.is_empty() {
        let _ = bot
            .send_message(chat_id, "⚠️ 没有可换 IP 的服务器")
            .await;
        return;
    }

    if changeable.len() == 1 {
        let r = changeable[0];
        do_reconnect(bot, chat_id, config, &r.router_id, &r.interface).await;
        return;
    }

    // 多台:内联键盘选择
    let buttons: Vec<Vec<InlineKeyboardButton>> = changeable
        .iter()
        .map(|r| {
            vec![InlineKeyboardButton::callback(
                r.label.clone(),
                format!("change:{}:{}", r.router_id, r.interface),
            )]
        })
        .collect();

    let _ = bot
        .send_message(chat_id, "选择要换 IP 的服务器：")
        .reply_markup(InlineKeyboardMarkup::new(buttons))
        .await;
}

async fn do_reconnect(
    bot: &Bot,
    chat_id: ChatId,
    config: &Config,
    router_id: &str,
    interface: &str,
) {
    let _ = bot.send_message(chat_id, "⏳ 开始换 IP，请稍候...").await;

    let result = async {
        let c = BoilClient::new()?;
        c.login(&config.boil_account, &config.boil_password).await?;

        let data = c.query_all().await?;
        let old_ip = data.get_ip(router_id, interface).map(str::to_string);

        anyhow::ensure!(
            data.daily_limit == 0 || data.daily_used < data.daily_limit,
            "今日额度已用尽（{}/{}），明日再试",
            data.daily_used,
            data.daily_limit
        );

        c.reconnect(router_id, interface).await?;

        sleep(Duration::from_secs(8)).await;

        let mut new_ip: Option<String> = None;
        for _ in 0..10u8 {
            let d2 = c.query_all().await?;
            let ip = d2.get_ip(router_id, interface).map(str::to_string);
            if ip.is_some() && ip != old_ip {
                new_ip = ip;
                break;
            }
            sleep(Duration::from_secs(3)).await;
        }

        anyhow::Ok((old_ip, new_ip))
    }
    .await;

    match result {
        Ok((old_ip, Some(new_ip))) => {
            let reachable = check_reachable(&new_ip).await;
            let (icon, reach) = if reachable {
                ("✅", "TCP 可达")
            } else {
                ("⚠️", "TCP 未通(可能有防火墙)")
            };
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "{icon} <b>换 IP 完成</b>\n旧 IP: <code>{}</code>\n新 IP: <code>{new_ip}</code> <i>{reach}</i>",
                        old_ip.as_deref().unwrap_or("未知"),
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await;
        }
        Ok((old_ip, None)) => {
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "⚠️ 重拨已触发，但未检测到 IP 变化\n旧 IP: <code>{}</code>\n请到面板手动确认",
                        old_ip.as_deref().unwrap_or("未知"),
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await;
        }
        Err(e) => {
            let _ = bot
                .send_message(chat_id, format!("❌ 换 IP 失败: {e}"))
                .await;
        }
    }
}

async fn check_reachable(ip: &str) -> bool {
    for port in [80u16, 443, 22] {
        if tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::TcpStream::connect(format!("{ip}:{port}")),
        )
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
        {
            return true;
        }
    }
    false
}
