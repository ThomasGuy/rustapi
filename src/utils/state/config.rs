use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub(crate) secret_key: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .expect("PORT must be a number"),
            secret_key: env::var("SECRET_KEY")
                .unwrap_or_else(|_| "twguy_kjf#hask~dfh^".to_string()),
        }
    }
}
