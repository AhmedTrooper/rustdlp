use crate::core::error::{DlpError, Result};
use reqwest::cookie::Jar;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct DlpConfig {
    pub proxy: Option<String>,
    pub cookies_path: Option<PathBuf>,
}

impl DlpConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_proxy(mut self, proxy: Option<String>) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn with_cookies(mut self, cookies: Option<PathBuf>) -> Self {
        self.cookies_path = cookies;
        self
    }

    pub fn build_http_client(&self) -> Result<reqwest::Client> {
        let mut builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30));

        if let Some(ref proxy_url) = self.proxy {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| DlpError::Network(e))?;
            builder = builder.proxy(proxy);
        }

        if let Some(ref cookie_file) = self.cookies_path {
            if cookie_file.exists() {
                let jar = Arc::new(Jar::default());
                Self::load_netscape_cookies(cookie_file, &jar)?;
                builder = builder.cookie_provider(jar);
            }
        }

        builder.build().map_err(DlpError::Network)
    }

    fn load_netscape_cookies(path: &Path, jar: &Jar) -> Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() || (trimmed.starts_with('#') && !trimmed.starts_with("#HttpOnly_")) {
                continue;
            }

            let cleaned = if trimmed.starts_with("#HttpOnly_") {
                &trimmed[10..]
            } else {
                trimmed
            };

            let parts: Vec<&str> = cleaned.split('\t').collect();
            if parts.len() >= 7 {
                let domain = parts[0];
                let _flag = parts[1];
                let path_str = parts[2];
                let secure = parts[3] == "TRUE";
                let _expiry = parts[4];
                let name = parts[5];
                let value = parts[6];

                let scheme = if secure { "https" } else { "http" };
                let host = domain.trim_start_matches('.');
                let cookie_str = format!("{}={}; Path={}; Domain={}", name, value, path_str, domain);

                if let Ok(url) = Url::parse(&format!("{}://{}/", scheme, host)) {
                    jar.add_cookie_str(&cookie_str, &url);
                }
            }
        }

        Ok(())
    }
}
