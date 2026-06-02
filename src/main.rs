mod boil;
mod bot;
mod cli;
mod config;
mod core;
mod service;
mod streaming;
mod timer;

use clap::{Parser, Subcommand};
use dialoguer::Select;

#[derive(Parser)]
#[command(name = "boil", about = "Boil.network 换 IP 工具", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 查看当前 IP 和今日剩余额度
    Status,
    /// 检查当前 IP 质量
    Check,
    /// 换 IP（重拨）
    Change,
    /// 后台守护进程：有 TG 则启动机器人，有 cron 则运行定时任务（系统服务使用此命令）
    Daemon,
    /// 启动 Telegram 机器人（需配置 TG）
    Bot,
    /// 重新运行配置向导
    Setup,
    /// 定时换 IP 设置，如: boil timer "0 */6 * * *" 或 boil timer off
    Timer {
        /// cron 表达式（5字段）或 "off"，留空查看当前设置
        expr: Option<String>,
    },
    /// 系统服务管理
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
}

#[derive(Subcommand)]
enum ServiceAction {
    /// 安装并启用 systemd 服务
    Install,
    /// 停止并卸载 systemd 服务
    Uninstall,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    match cli.command {
        None => {
            let config = config::load_or_setup().await?;
            interactive_menu(&config).await?;
        }
        Some(Commands::Status) => {
            let config = config::load_or_setup().await?;
            cli::cmd_status(&config).await?;
        }
        Some(Commands::Check) => {
            let config = config::load_or_setup().await?;
            cli::cmd_check(&config).await?;
        }
        Some(Commands::Change) => {
            let config = config::load_or_setup().await?;
            cli::cmd_change(&config).await?;
        }
        Some(Commands::Daemon) => {
            let config = config::load_or_setup().await?;
            run_daemon(config).await?;
        }
        Some(Commands::Bot) => {
            let config = config::load_or_setup().await?;
            bot::run(config).await?;
        }
        Some(Commands::Setup) => {
            config::run_setup_wizard().await?;
        }
        Some(Commands::Timer { expr }) => {
            let config = config::load_or_setup().await?;
            cli::cmd_timer(&config, expr.as_deref().unwrap_or(""))?;
        }
        Some(Commands::Service { action }) => match action {
            ServiceAction::Install => service::install()?,
            ServiceAction::Uninstall => service::uninstall()?,
        },
    }

    Ok(())
}

/// 系统服务入口：有 TG 跑 bot（含定时器），只有 cron 跑纯定时器，都没有则报错
async fn run_daemon(config: config::Config) -> anyhow::Result<()> {
    let has_tg = config.has_tg();
    let has_cron = config.change_cron.is_some();

    anyhow::ensure!(
        has_tg || has_cron,
        "守护进程无事可做：请配置 Telegram Bot 或定时换 IP（boil timer \"0 */6 * * *\"）"
    );

    if has_tg {
        // bot::run 内部已处理 cron，一起跑
        bot::run(config).await?;
    } else {
        // 只有 cron，纯定时模式
        println!("定时换 IP 模式启动，cron: {}", config.change_cron.as_deref().unwrap());
        use std::sync::Arc;
        let cfg = Arc::new(config);
        let _sched = timer::start(cfg).await?;
        // 阻塞保持进程存活
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    Ok(())
}

fn interactive_timer(config: &config::Config) -> anyhow::Result<()> {
    use dialoguer::Input;

    let current = config.change_cron.as_deref().unwrap_or("未启用");
    println!("⏰ 当前定时换 IP: {current}");
    println!("  输入 cron 表达式设置定时（如 0 */6 * * *），输入 off 关闭，直接回车取消");

    let input: String = Input::new()
        .with_prompt("定时设置")
        .allow_empty(true)
        .interact_text()?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        println!("已取消");
    } else {
        cli::cmd_timer(config, trimmed)?;
    }
    Ok(())
}

async fn interactive_menu(config: &config::Config) -> anyhow::Result<()> {
    let items = vec![
        "📡  status   查看当前 IP",
        "🔍  check    检查 IP 质量和流媒体解锁",
        "🔄  change   换 IP",
        "⏰  timer    查看/设置定时换 IP",
        "⚙️   setup    重新配置",
        "❌  退出",
    ];

    loop {
        let idx = Select::new()
            .with_prompt("Boil — 选择操作")
            .items(&items)
            .default(0)
            .interact()?;

        match idx {
            0 => cli::cmd_status(config).await?,
            1 => cli::cmd_check(config).await?,
            2 => cli::cmd_change(config).await?,
            3 => interactive_timer(config)?,
            4 => {
                config::run_setup_wizard().await?;
                break;
            }
            _ => break,
        }
    }

    Ok(())
}
