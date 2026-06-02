mod boil;
mod bot;
mod cli;
mod config;
mod core;
mod service;
mod streaming;
mod timer;

use clap::{Parser, Subcommand};

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
            if config.has_tg() {
                println!("启动 Telegram 机器人...");
                bot::run(config).await?;
            } else {
                println!("提示: 未配置 Telegram，可直接使用以下命令：");
                println!("  boil status        查看当前 IP");
                println!("  boil check         检查 IP 质量");
                println!("  boil change        换 IP");
                println!("  boil timer         查看/设置定时换 IP");
                println!("  boil setup         重新配置（含 TG）");
            }
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
