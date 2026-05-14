use std::env;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Environment {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "production")]
    Production,
}

impl Environment {
    // Helper method to explicitly return if the environment requires HTTPS secure cookies
    pub fn requires_secure_cookies(&self) -> bool {
        match self {
            Environment::Local => false,
            Environment::Production => true,
        }
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub(crate) secret_key: String,
    pub app_env: Environment,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let env_str = env::var("ENVIRONMANT").unwrap_or_else(|_| "local".to_string());
        let environment: Environment = serde_json::from_value(serde_json::json!(env_str))
            .expect("ENVIRONMENT variable must be either 'local' or 'production'");

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .expect("PORT must be a number"),
            secret_key: env::var("SECRET_KEY")
                .unwrap_or_else(|_| "twguy_kjf#hask~dfh^".to_string()),
            app_env: environment,
        }
    }
}
