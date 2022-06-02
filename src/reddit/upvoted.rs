use async_recursion::async_recursion;
use futures_core::stream::Stream;
use roux::me::responses::SavedData;
use roux::util::FeedOption;

use async_stream::try_stream;
use roux::{util::RouxError, Me};

use super::{Config, RedditCredentials, RedditPage};

#[derive(Debug, Clone)]
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
            let mut pagination = FeedOption::new().count(0).limit(self.config.reddit.page_size);

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
    async fn fetch_next_page(
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpvotedItem {
    pub author: String,
    pub subreddit: String,
    pub html: Option<String>,
    pub text: String,
    pub url: String,
    pub content_url: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::super::configs::Config;
    use super::{RedditCredentials, UpvotedItem, Upvotes};

    #[tokio::test]
    async fn upvotes_async_stream_yields_upvoted_item() {
        use futures_util::pin_mut;
        use futures_util::stream::StreamExt;
        use std::env;

        let config = Config::default();
        let mut upvotes = Upvotes {
            config: &config,
            credentials: RedditCredentials {
                username: env::var("REDDIT_USERNAME").expect("SHIT?"),
                password: env::var("REDDIT_PASSWORD").expect("SHIT?"),
            },
            me: None,
        };

        dbg!(&upvotes);

        let stream = upvotes.as_stream();
        pin_mut!(stream);

        let first_upvoted_item: UpvotedItem = stream
            .take(1)
            .map(|upvoted_item| upvoted_item.unwrap_or_else(|_| panic!("there was an error 😱")))
            .next()
            .await
            .unwrap();

        // NOTE: this assert depends on a specific set of reddit credentials since this test
        // does not call a mocked version of the api because mocking out the reddit client
        // library we're using (Roux) is a big pain.
        // And... this test might flake out for the following reasons: 😱😱😱
        //     - transient network issues or the data in
        //     - this reddit submission changing
        //     - this account upvotes a new submission, or un-upvotes this one
        assert_eq!(
            dbg!(first_upvoted_item),
            UpvotedItem {
                author: "42jd".to_owned(),
                subreddit: "NixOS".to_owned(),
                html: None,
                text: "".to_owned(),
                url: "/r/NixOS/comments/tvi0eq/bare_bones_systemdbased_initrd_merged/".to_owned(),
                content_url: Some(
                    "https://github.com/NixOS/nixpkgs/pull/164943#event-6358295283=".to_owned()
                ),
            }
        )
    }
}
