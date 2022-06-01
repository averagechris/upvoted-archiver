use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use std::env;
use upvoted_archiver::{Config, RedditCredentials, Upvotes};

#[cfg(test)]
#[tokio::test]
async fn test_me() {
    let config = Config::default();
    let mut upvotes = Upvotes {
        config: &config,
        credentials: RedditCredentials::new(
            &env::var("REDDIT_USERNAME").expect("SHIT?"),
            &env::var("REDDIT_PASSWORD").expect("SHIT?"),
        ),
        me: None,
    };

    let stream = upvotes.as_stream();
    pin_mut!(stream);
    let mut trunc_stream = stream.take(15);

    while let Some(thing) = trunc_stream.next().await {
        dbg!(thing).unwrap();
    }
}
