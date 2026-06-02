use anyhow::Context as _;
use dialoguer::{Input, Password};
use std::{io::BufRead as _, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub boil_account: String,
    pub boil_password: String,
    pub tg_token: String,
    pub tg_chat_id: String,
}

fn config_path() -> PathBuf {
    // 优先查找可执行文件同目录下的 config.env
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
        tg_token: std::env::var("TG_TOKEN").context("缺少 TG_TOKEN 配置")?,
        tg_chat_id: std::env::var("TG_CHAT_ID").context("缺少 TG_CHAT_ID 配置")?,
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

async fn run_setup_wizard() -> anyhow::Result<()> {
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
        let ip = data
            .get_ip(&item.router_id, &item.interface)
            .unwrap_or("未知");
        let tag = if item.nat_no_change {
            "NAT 不可换"
        } else {
            "可换 IP ✅"
        };
        println!("  {} | IP: {} | {}", item.label, ip, tag);
    }
    println!();

    let tg_token: String = Input::new()
        .with_prompt("Telegram Bot Token（从 @BotFather 获取）")
        .interact_text()?;

    println!("\n请向你的机器人发送任意一条消息，然后按回车继续...");
    let stdin = std::io::stdin();
    stdin.lock().lines().next();

    let chat_id = detect_chat_id(&tg_token)
        .await
        .context("未检测到 chat_id，请确认已向机器人发送消息")?;
    println!("✅ 检测到 chat_id: {}\n", chat_id);

    let content = format!(
        "BOIL_ACCOUNT='{}'\nBOIL_PASSWORD='{}'\nTG_TOKEN='{}'\nTG_CHAT_ID='{}'\n",
        account,
        password.replace('\'', "'\\''"),
        tg_token,
        chat_id,
    );
    std::fs::write("config.env", content)?;
    println!("✅ 配置已保存到 config.env\n");

    Ok(())
}

async fn detect_chat_id(token: &str) -> anyhow::Result<String> {
    let url = format!(
        "https://api.telegram.org/bot{}/getUpdates?offset=-1&limit=1",
        token
    );
    let resp: serde_json::Value = reqwest::get(&url)
        .await?
        .json()
        .await?;

    resp["result"][0]["message"]["from"]["id"]
        .as_i64()
        .map(|id| id.to_string())
        .context("未检测到消息")
}
