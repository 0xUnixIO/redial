use std::{fs, path::Path, process::Command};

const SERVICE_NAME: &str = "boil";
const SERVICE_PATH: &str = "/etc/systemd/system/boil.service";
const CONFIG_DIR: &str = "/etc/boil";

pub fn install() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();

    // 配置目录
    fs::create_dir_all(CONFIG_DIR)?;

    // 如果当前目录有 config.env，复制过去
    if Path::new("config.env").exists() && !Path::new("/etc/boil/config.env").exists() {
        fs::copy("config.env", "/etc/boil/config.env")?;
        println!("✅ 已复制 config.env 到 {CONFIG_DIR}/config.env");
    }

    let unit = format!(
        r#"[Unit]
Description=Boil IP Bot
After=network.target

[Service]
ExecStart={exe_str}
WorkingDirectory={CONFIG_DIR}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#
    );

    fs::write(SERVICE_PATH, unit)?;
    println!("✅ 已写入 {SERVICE_PATH}");

    run_systemctl(&["daemon-reload"])?;
    run_systemctl(&["enable", "--now", SERVICE_NAME])?;
    println!("✅ 服务已启动，开机自启已启用");
    println!("\n常用命令:");
    println!("  systemctl status  {SERVICE_NAME}");
    println!("  systemctl restart {SERVICE_NAME}");
    println!("  journalctl -fu    {SERVICE_NAME}");
    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    run_systemctl(&["disable", "--now", SERVICE_NAME])?;
    if Path::new(SERVICE_PATH).exists() {
        fs::remove_file(SERVICE_PATH)?;
    }
    run_systemctl(&["daemon-reload"])?;
    println!("✅ 服务已卸载");
    Ok(())
}

fn run_systemctl(args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("systemctl").args(args).status()?;
    anyhow::ensure!(status.success(), "systemctl {} 失败", args.join(" "));
    Ok(())
}
