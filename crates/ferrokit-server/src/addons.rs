//! Optional add-on containers: where they live, whether they're up, and what they can do.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde_json::{json, Value};

use crate::Config;

/// How often the core asks each add-on how it is.
pub const CHECK_EVERY: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Addon {
    Office,
    Ocr,
}

impl Addon {
    pub const ALL: [Addon; 2] = [Addon::Office, Addon::Ocr];

    pub fn name(self) -> &'static str {
        match self {
            Addon::Office => "office",
            Addon::Ocr => "ocr",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Addon::Office => "Office add-on",
            Addon::Ocr => "OCR add-on",
        }
    }

    pub fn start_hint(self) -> String {
        format!("docker compose --profile {} up -d", self.name())
    }

/// How long the worker may take to start answering: a backstop for a worker that never answers.
    /// The worker has its own, shorter limits (2 minutes per Office file, 30 minutes per OCR request)
    /// and sends a proper message when one is hit; a large Office batch can take longer than 2 minutes
    /// in total, so the backstop is the same 30 minutes for both.
    pub fn limit(self) -> Duration {
        match self {
            Addon::Office | Addon::Ocr => Duration::from_secs(30 * 60 + 30),
        }
    }

    pub fn url(self, config: &Config) -> &str {
        match self {
            Addon::Office => &config.office_url,
            Addon::Ocr => &config.ocr_url,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub ready: bool,
    /// The worker's `office` or `ocr` section from its `/health`, e.g. `{"ops": [...]}`.
    pub info: Value,
}

/// The latest known status of every add-on, shared by all requests.
#[derive(Clone, Default)]
pub struct Registry(Arc<RwLock<HashMap<Addon, Status>>>);

impl Registry {
    pub fn set(&self, addon: Addon, status: Status) {
        if let Ok(mut m) = self.0.write() {
            m.insert(addon, status);
        }
    }

    pub fn mark_down(&self, addon: Addon) {
        self.set(addon, Status::default());
    }

    /// `{"office": {"ready": bool, "ops": [...]}, "ocr": {"ready": bool, "ops": [...], "languages": [...]}}`
    pub fn snapshot(&self) -> Value {
        let m = self.0.read().map(|m| m.clone()).unwrap_or_default();
        let mut out = serde_json::Map::new();
        for addon in Addon::ALL {
            let s = m.get(&addon).cloned().unwrap_or_default();
            let mut entry = match (&s.info, s.ready) {
                (Value::Object(o), true) => o.clone(),
                _ => serde_json::Map::new(),
            };
            entry.insert("ready".into(), json!(s.ready));
            entry.entry("ops").or_insert(json!([]));
            out.insert(addon.name().into(), Value::Object(entry));
        }
        Value::Object(out)
    }

    /// Ask every add-on for its `/health` once, all at the same time.
    pub async fn check_once(&self, client: &reqwest::Client, config: &Config) {
        tokio::join!(self.check(client, config, Addon::Office), self.check(client, config, Addon::Ocr));
    }

    async fn check(&self, client: &reqwest::Client, config: &Config, addon: Addon) {
        let url = format!("{}/health", addon.url(config).trim_end_matches('/'));
        let status = match client.get(&url).timeout(Duration::from_secs(3)).send().await {
            Ok(res) if res.status().is_success() => match res.json::<Value>().await {
                Ok(health) => {
                    let info = health.get(addon.name()).cloned().unwrap_or(Value::Null);
                    let has_ops = info.get("ops").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
                    Status { ready: has_ops, info }
                }
                Err(_) => Status::default(),
            },
            _ => Status::default(),
        };
        let was = self.0.read().ok().and_then(|m| m.get(&addon).map(|s| s.ready));
        if was != Some(status.ready) {
            tracing::info!(addon = addon.name(), ready = status.ready, "add-on status");
        }
        self.set(addon, status);
    }

    /// Keep checking in the background for as long as the server runs.
    pub fn watch(self, client: reqwest::Client, config: Arc<Config>) {
        tokio::spawn(async move {
            // Every 15 s from the start of one check to the next, however long a check takes.
            let mut every = tokio::time::interval(CHECK_EVERY);
            every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                every.tick().await;
                self.check_once(&client, &config).await;
            }
        });
    }
}
