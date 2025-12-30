use std::{fs::File, io::Read, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

const DEFAULT_HTTP_PORT: u16 = 9009;
fn default_http_port() -> u16 { DEFAULT_HTTP_PORT }

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalConfig {
    pub server_enable: bool,
    pub port: u16,
    pub tls_cert: Option<String>,
    pub tls_priv: Option<String>,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self { 
            server_enable: false, 
            port: DEFAULT_HTTP_PORT,
            tls_cert: None,
            tls_priv: None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerItemConfig {
    pub name: String,
    pub remote: String,
    pub identity: String,
    pub key: String,
    #[serde(default = "default_http_port")]
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Configuration {
    #[serde(default)]
    pub local: LocalConfig,
    #[serde(default)]
    pub servers: Vec<ServerItemConfig>
}

impl Configuration {
    pub fn from_file(toml_config: &str) -> Result<Self> {
        let mut f = File::open(Path::new(toml_config)).unwrap();
        let mut file_content = String::new();
        let _ = f.read_to_string(&mut file_content).unwrap();

        let raw_config = toml::from_str(file_content.as_str());
        let config = raw_config.unwrap_or_default();

        Ok(config)
    }
    //TODO: generate default template
}
