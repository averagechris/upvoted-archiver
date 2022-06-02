#![allow(dead_code)]
mod configs;
mod upvoted;

pub use configs::Config;
pub use upvoted::{UpvotedItem, Upvotes};

use roux::me::responses::SavedData;
use roux::responses::{BasicThing, Listing};
use roux::Reddit;
use roux::{util::RouxError, Me};

type RedditPage = BasicThing<Listing<BasicThing<SavedData>>>;

#[derive(Clone)]
pub struct RedditCredentials {
    pub username: String,
    pub password: String,
}

impl RedditCredentials {
    pub fn user_agent(&self, config: &Config) -> String {
        config.user_agent(&self.username)
    }

    async fn login(&self, config: &Config) -> Result<Me, RouxError> {
        Reddit::new(
            &config.user_agent(&self.username),
            &config.reddit.client_id,
            &config.reddit.client_secret,
        )
        .username(&self.username)
        .password(&self.password)
        .login()
        .await
    }
}

impl std::fmt::Debug for RedditCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedditCredentials")
            .field(
                "username",
                &if self.username.is_empty() {
                    "[MISSING]"
                } else {
                    "[omitted]"
                },
            )
            .field(
                "password",
                &if self.password.is_empty() {
                    "[MISSING]"
                } else {
                    "[omitted]"
                },
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::configs::RedditConfig;
    use super::{Config, RedditCredentials};

    #[test]
    fn reddit_credentials_debug_fmt() {
        let empty = RedditCredentials {
            username: "".to_owned(),
            password: "".to_owned(),
        };

        let not_empty = RedditCredentials {
            username: "hello".to_owned(),
            password: "world".to_owned(),
        };

        let split_empty = RedditCredentials {
            username: "hello".to_owned(),
            password: "".to_owned(),
        };

        assert_eq!(
            "RedditCredentials { username: \"[MISSING]\", password: \"[MISSING]\" }",
            format!("{empty:?}")
        );
        assert_eq!(
            "RedditCredentials { username: \"[omitted]\", password: \"[omitted]\" }",
            format!("{not_empty:?}")
        );
        assert_eq!(
            "RedditCredentials { username: \"[omitted]\", password: \"[MISSING]\" }",
            format!("{split_empty:?}")
        );
    }

    #[test]
    fn reddeit_credentials_user_agent() {
        let creds = RedditCredentials {
            username: "username".to_owned(),
            password: "".to_owned(),
        };
        let config = Config {
            reddit: RedditConfig {
                client_id: "client_id".to_owned(),
                client_secret: "client_secret".to_owned(),
                page_size: 100,
            },
            os_arch: "os_arch",
            os_name: "os_name",
            app_version: "0.0.1",
        };

        assert_eq!(
            "os_arch-os_name:upvoted_archiver:0.0.1 (by /u/username)",
            creds.user_agent(&config)
        );
    }
}
