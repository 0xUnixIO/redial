use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Context as _;
use reqwest::cookie::Jar;
use serde::Deserialize;

pub struct BoilClient {
    client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
pub struct QueryAllResponse {
    pub daily_limit: i64,
    pub daily_used: i64,
    pub results: HashMap<String, HashMap<String, String>>,
    pub zone_items: Vec<ZoneItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ZoneItem {
    pub router_id: String,
    pub interface: String,
    pub label: String,
    pub nat_no_change: bool,
    pub status: String,
}

impl QueryAllResponse {
    pub fn get_ip(&self, router_id: &str, interface: &str) -> Option<&str> {
        self.results
            .get(router_id)?
            .get(interface)
            .map(String::as_str)
    }

    pub fn changeable(&self) -> Vec<&ZoneItem> {
        self.zone_items
            .iter()
            .filter(|r| !r.nat_no_change && r.status == "ok")
            .collect()
    }
}

impl BoilClient {
    pub fn new() -> anyhow::Result<Self> {
        let jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(jar)
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { client })
    }

    pub async fn login(&self, account: &str, password: &str) -> anyhow::Result<()> {
        let resp = self
            .client
            .post("https://ippanel.boil.network/login")
            .form(&[("account", account), ("password", password)])
            .send()
            .await
            .context("登录请求失败")?;

        anyhow::ensure!(
            resp.status().is_success() || resp.status().as_u16() == 302,
            "登录失败: HTTP {}",
            resp.status()
        );
        Ok(())
    }

    pub async fn query_all(&self) -> anyhow::Result<QueryAllResponse> {
        self.client
            .post("https://ippanel.boil.network/api/query_all")
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("query_all 请求失败")?
            .json::<QueryAllResponse>()
            .await
            .context("query_all 响应解析失败")
    }

    pub async fn reconnect(&self, router_id: &str, interface: &str) -> anyhow::Result<()> {
        self.client
            .post("https://ippanel.boil.network/api/reconnect")
            .json(&serde_json::json!({
                "router_id": router_id,
                "interface": interface
            }))
            .send()
            .await
            .context("reconnect 请求失败")?;
        Ok(())
    }
}
