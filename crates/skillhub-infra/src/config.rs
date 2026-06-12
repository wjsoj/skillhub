use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    pub publish: PublishConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub public_base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum StorageConfig {
    Local { root: String },
    S3 {
        bucket: String,
        region: String,
        endpoint: Option<String>,
        access_key: String,
        secret_key: String,
        path_style: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_prefix: String,
    pub bootstrap_admin_enabled: bool,
    pub bootstrap_admin_username: String,
    pub bootstrap_admin_password: String,
    pub oauth: Vec<OauthProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OauthProviderConfig {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub redirect_url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishConfig {
    pub allowed_file_extensions: Vec<String>,
    pub max_package_size_mb: u64,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(
                // SKILLHUB__SERVER__HOST → server.host
                // Both the prefix→key split and the nested separator
                // use "__"; everything else (single `_`) stays inside
                // the field name itself (e.g. `max_connections`).
                config::Environment::with_prefix("SKILLHUB")
                    .prefix_separator("__")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;
        Ok(cfg.try_deserialize()?)
    }
}
