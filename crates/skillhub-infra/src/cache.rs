use redis::aio::ConnectionManager;

#[derive(Clone)]
pub struct RedisClient {
    pub conn: ConnectionManager,
}

impl RedisClient {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }
}
