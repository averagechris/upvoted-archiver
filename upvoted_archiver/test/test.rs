#[cfg(test)]
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use std::env;
use upvoted_archiver::{Config, RedditCredentials, UpvotedItem, Upvotes};

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
        client_id: "client_id".to_owned(),
        client_secret: "client_secret".to_owned(),
        os_arch: "os_arch",
        os_name: "os_name",
        upvoted_archiver_version: "0.0.1",
        page_size: 100,
    };

    assert_eq!(
        "os_arch-os_name:upvoted_archiver:0.0.1 (by /u/username)",
        creds.user_agent(&config)
    );
}

#[tokio::test]
async fn upvotes_async_stream_yields_upvoted_item() {
    let config = Config::default();
    let mut upvotes = Upvotes {
        config: &config,
        credentials: RedditCredentials::new(
            &env::var("REDDIT_USERNAME").expect("SHIT?"),
            &env::var("REDDIT_PASSWORD").expect("SHIT?"),
        ),
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
