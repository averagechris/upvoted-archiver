use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub reddit: RedditConfig,
    pub os_arch: &'static str,
    pub os_name: &'static str,
    pub app_version: &'static str,
}

impl Config {
    pub fn user_agent(&self, reddit_username: &str) -> String {
        let arch = self.os_arch;
        let os_name = self.os_name;
        let app_version = self.app_version;
        format!("{arch}-{os_name}:upvoted_archiver:{app_version} (by /u/{reddit_username})")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            reddit: RedditConfig::default(),
            os_arch: std::env::consts::ARCH,
            os_name: std::env::consts::OS,
            app_version: env!("CARGO_PKG_VERSION"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RedditConfig {
    pub client_id: String,
    pub client_secret: String,
    pub page_size: u32,
}

impl Default for RedditConfig {
    fn default() -> Self {
        let id_var = "UPVOTED_ARCHIVER_REDDIT_CLIENT_ID";
        let secret_var = "UPVOTED_ARCHIVER_REDDIT_CLIENT_SECRET";
        Self {
            client_id: env::var(id_var)
                .unwrap_or_else(|_| panic!("missing required env var: {id_var}")),
            client_secret: env::var(secret_var)
                .unwrap_or_else(|_| panic!("missing required env var: {secret_var}")),
            page_size: 100,
        }
    }
}
