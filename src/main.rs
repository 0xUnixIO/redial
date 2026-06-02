mod boil;
mod bot;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cfg = config::load_or_setup().await?;
    println!("配置加载成功，启动 Telegram 机器人...");
    bot::run(cfg).await
}
