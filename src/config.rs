use anyhow::Context as _;
use dialoguer::{Input, Password, Select};
use std::{io::BufRead as _, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub boil_account: String,
    pub boil_password: String,
    pub tg_token: Option<String>,
    pub tg_chat_id: Option<String>,
    /// 定时换 IP 的 cron 表达式（5字段），None 表示不启用
    pub change_cron: Option<String>,
}

impl Config {
    pub fn has_tg(&self) -> bool {
        self.tg_token.is_some() && self.tg_chat_id.is_some()
    }
}

/// 验证 cron 表达式是否合法（5字段：min hour day month weekday）
pub fn validate_cron(expr: &str) -> anyhow::Result<()> {
    use tokio_cron_scheduler::Job;
    // tokio-cron-scheduler 用 6字段（加秒），我们在前面补 0 秒
    let full = format!("0 {}", expr.trim());
    Job::new(&full, |_, _| {}).map_err(|e| anyhow::anyhow!("cron 表达式无效: {e}"))?;
    Ok(())
}

/// 将 cron 表达式写入 config.env（None 表示清除）
pub fn save_cron(cron: Option<&str>) -> anyhow::Result<()> {
    let path = config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();

    let filtered: String = content
        .lines()
        .filter(|l| !l.starts_with("CHANGE_CRON="))
        .map(|l| format!("{l}\n"))
        .collect();

    let new_content = match cron {
        Some(expr) => format!("{filtered}CHANGE_CRON='{expr}'\n"),
        None => filtered,
    };
    std::fs::write(&path, new_content)?;
    Ok(())
}

fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.env")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("config.env"))
}

pub fn load() -> anyhow::Result<Config> {
    let path = config_path();
    if path.exists() {
        dotenvy::from_path(&path).ok();
    }
    dotenvy::dotenv().ok();

    Ok(Config {
        boil_account: std::env::var("BOIL_ACCOUNT").context("缺少 BOIL_ACCOUNT 配置")?,
        boil_password: std::env::var("BOIL_PASSWORD").context("缺少 BOIL_PASSWORD 配置")?,
        tg_token: std::env::var("TG_TOKEN").ok(),
        tg_chat_id: std::env::var("TG_CHAT_ID").ok(),
        change_cron: std::env::var("CHANGE_CRON").ok(),
    })
}

pub async fn load_or_setup() -> anyhow::Result<Config> {
    match load() {
        Ok(cfg) => Ok(cfg),
        Err(_) => {
            println!("未找到配置，启动首次配置向导...\n");
            run_setup_wizard().await?;
            load()
        }
    }
}

pub async fn run_setup_wizard() -> anyhow::Result<()> {
    let account: String = Input::new()
        .with_prompt("Boil 账号（邮箱）")
        .interact_text()?;

    let password: String = Password::new()
        .with_prompt("Boil 密码")
        .interact()?;

    println!("\n测试登录中...");
    let client = crate::boil::BoilClient::new()?;
    client
        .login(&account, &password)
        .await
        .context("登录失败，请检查账号密码")?;

    let data = client.query_all().await?;
    println!("✅ 登录成功，找到以下服务器：\n");
    for item in &data.zone_items {
        let ip = data.get_ip(&item.router_id, &item.interface).unwrap_or("未知");
        let tag = if item.nat_no_change { "NAT 不可换" } else { "可换 IP ✅" };
        println!("  {} | IP: {} | {}", item.label, ip, tag);
    }
    println!();

    // TG 可选
    let want_tg = Select::new()
        .with_prompt("配置 Telegram Bot（用于远程控制）")
        .items(&["是，现在配置", "否，跳过（之后可用 boil setup 补充）"])
        .default(0)
        .interact()? == 0;

    let (tg_token, tg_chat_id) = if want_tg {
        let token: String = Input::new()
            .with_prompt("Bot Token（从 @BotFather 获取）")
            .interact_text()?;

        println!("\n请向你的机器人发送任意消息，然后按回车继续...");
        std::io::stdin().lock().lines().next();

        let chat_id = detect_chat_id(&token)
            .await
            .context("未检测到 chat_id，请确认已向机器人发送消息")?;
        println!("✅ 检测到 chat_id: {chat_id}\n");
        (Some(token), Some(chat_id))
    } else {
        println!("已跳过 Telegram 配置，可使用 boil status/change 命令行操作\n");
        (None, None)
    };

    let mut content = format!(
        "BOIL_ACCOUNT='{}'\nBOIL_PASSWORD='{}'\n",
        account,
        password.replace('\'', "'\\''"),
    );
    if let (Some(token), Some(chat_id)) = (tg_token, tg_chat_id) {
        content.push_str(&format!("TG_TOKEN='{}'\nTG_CHAT_ID='{}'\n", token, chat_id));
    }

    std::fs::write("config.env", content)?;
    println!("✅ 配置已保存到 config.env\n");
    println!("常用命令:");
    println!("  boil status    查看当前 IP");
    println!("  boil check     检查 IP 质量和流媒体解锁");
    println!("  boil change    换 IP");
    println!();
    Ok(())
}

async fn detect_chat_id(token: &str) -> anyhow::Result<String> {
    let url = format!(
        "https://api.telegram.org/bot{}/getUpdates?offset=-1&limit=1",
        token
    );
    let resp: serde_json::Value = reqwest::get(&url).await?.json().await?;
    resp["result"][0]["message"]["from"]["id"]
        .as_i64()
        .map(|id| id.to_string())
        .context("未检测到消息")
}
