#![allow(dead_code)]
use async_recursion::async_recursion;
use futures_core::stream::Stream;
use roux::util::FeedOption;
use std::env;

use async_stream::try_stream;
use roux::me::responses::SavedData;
use roux::responses::{BasicThing, Listing};
use roux::{util::RouxError, Me, Reddit};

type RedditPage = BasicThing<Listing<BasicThing<SavedData>>>;

pub struct Config {
    client_id: String,
    client_secret: String,
    os_arch: &'static str,
    os_name: &'static str,
    upvoted_archiver_version: &'static str,
    page_size: u32,
}

impl Default for Config {
    fn default() -> Self {
        let id_var = "UPVOTED_ARCHIVER_REDDIT_CLIENT_ID";
        let secret_var = "UPVOTED_ARCHIVER_REDDIT_CLIENT_SECRET";

        Self {
            client_id: env::var(id_var)
                .unwrap_or_else(|_| panic!("missing required env var: {id_var}")),
            client_secret: env::var(secret_var)
                .unwrap_or_else(|_| panic!("missing required env var: {secret_var}")),
            os_arch: std::env::consts::ARCH,
            os_name: std::env::consts::OS,
            upvoted_archiver_version: env!("CARGO_PKG_VERSION"),
            page_size: 100,
        }
    }
}

pub struct RedditCredentials {
    username: String,
    password: String,
}

impl RedditCredentials {
    pub fn new<'a>(username: &'a str, password: &'a str) -> Self {
        Self {
            username: username.to_owned(),
            password: password.to_owned(),
        }
    }

    async fn login(&self, config: &Config) -> Result<Me, RouxError> {
        let arch = config.os_arch;
        let os_name = config.os_name;
        let app_version = config.upvoted_archiver_version;
        let username = &self.username;
        let user_agent =
            format!("{arch}-{os_name}:upvoted_archiver:{app_version} (by /u/{username})");

        Reddit::new(&user_agent, &config.client_id, &config.client_secret)
            .username(&self.username)
            .password(&self.password)
            .login()
            .await
    }
}

pub struct Upvotes<'a> {
    pub credentials: RedditCredentials,
    pub config: &'a Config,
    pub me: Option<Me>,
}

impl<'a> Upvotes<'a> {
    pub fn new(credentials: RedditCredentials, config: &'a Config) -> Self {
        Self {
            credentials,
            config,
            me: None,
        }
    }

    pub fn as_stream(&'a mut self) -> impl Stream<Item = Result<UpvotedItem, RouxError>> + 'a {
        try_stream! {
            let mut pagination = FeedOption::new().count(0).limit(self.config.page_size);

            loop {
                let page = self.fetch_next_page(Some(pagination.clone())).await?;

                pagination = match page.data.after {
                    Some(after) => pagination.after(&after),
                    None => pagination
                }.count(page.data.children.len() as u32);

                for item in page.data.children {
                    yield UpvotedItem::from(item.data)
                }
            }
        }
    }

    #[async_recursion]
    pub async fn fetch_next_page(
        &mut self,
        pagination: Option<FeedOption>,
    ) -> Result<RedditPage, RouxError> {
        match &self.me {
            Some(me) => me.upvoted(pagination).await,
            None => {
                self.login().await?;
                self.fetch_next_page(pagination).await
            }
        }
    }

    async fn login(&mut self) -> Result<(), RouxError> {
        let me = self.credentials.login(self.config).await?;
        self.me = Some(me);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UpvotedItem {
    author: String,
    subreddit: String,
    html: Option<String>,
    text: String,
    url: String,
    content_url: Option<String>,
}

impl From<SavedData> for UpvotedItem {
    fn from(item: SavedData) -> Self {
        let missing_data = || "[deleted]".to_string();

        match item {
            SavedData::Submission(post_data) => Self {
                author: post_data.author,
                subreddit: post_data.subreddit,
                html: post_data.selftext_html,
                text: post_data.selftext,
                url: post_data.permalink,
                content_url: post_data.url,
            },
            SavedData::Comment(comment_data) => Self {
                author: comment_data.author.unwrap_or_else(missing_data),
                subreddit: comment_data.subreddit.unwrap_or_else(missing_data),
                html: comment_data.body_html,
                text: comment_data.body.unwrap_or_else(missing_data),
                url: comment_data.permalink.unwrap_or_else(missing_data),
                content_url: comment_data.link_url,
            },
        }
    }
}
