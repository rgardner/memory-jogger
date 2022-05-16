//! Utilities (e.g. HN, Reddit, and Wayback Machine API wrappers)
//!
//! - HN documentation: <https://hn.algolia.com/api>
//! - Reddit documentation: <https://www.reddit.com/dev/api>
//! - Wayback Machine documentation: <https://archive.org/help/wayback_api.php>

use std::fmt::Display;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use reqwest::Url;
use serde::Deserialize;

static HN_ITEM_URL: &str = "https://hacker-news.firebaseio.com/v0/item/";
static HN_SEARCH_URL: &str = "https://hn.algolia.com/api/v1/search";
static WAYBACK_URL: &str = "http://archive.org/wayback/available";

/// Finds submission URL for a given HN item or Reddit post.
pub(crate) async fn resolve_submission_url(
    url: Url,
    http_client: &reqwest::Client,
) -> Result<Option<String>> {
    let domain = if let Some(domain) = url.domain() {
        domain
    } else {
        return Ok(None);
    };
    if domain == "news.ycombinator.com" {
        resolve_hn_submission_url(url, http_client).await
    } else if domain == "www.reddit.com" {
        resolve_reddit_submission_url(url, http_client).await
    } else {
        Ok(None)
    }
}

#[derive(Deserialize)]
struct HnItemResponse {
    url: Option<String>,
}

/// Finds submission URL for a given HN item.
async fn resolve_hn_submission_url(
    url: Url,
    http_client: &reqwest::Client,
) -> Result<Option<String>> {
    let post_id = url.query_pairs().find(|(key, _)| key == "id").unwrap().1;
    let api_url = Url::parse(HN_ITEM_URL)
        .unwrap()
        .join(format!("{}.json", post_id).as_str())
        .unwrap();
    let resp = http_client
        .get(api_url)
        .send()
        .await?
        .json::<HnItemResponse>()
        .await?;
    Ok(resp.url)
}

#[derive(Deserialize, Debug, PartialEq)]
struct RedditSubmissionListing {
    data: RedditSubmissionListingData,
}

#[derive(Deserialize, Debug, PartialEq)]
struct RedditSubmissionListingData {
    children: Vec<RedditSubmissionChild>,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(tag = "kind", content = "data")]
enum RedditSubmissionChild {
    #[serde(rename = "t1")]
    Comment {},
    #[serde(rename = "t3")]
    Link { url: String },
}

/// Finds submission URL for a given Reddit post.
async fn resolve_reddit_submission_url(
    url: Url,
    http_client: &reqwest::Client,
) -> Result<Option<String>> {
    let url = url.join(".json").unwrap();
    let resp = http_client
        .get(url.clone())
        .send()
        .await?
        .json::<Vec<RedditSubmissionListing>>()
        .await
        .with_context(|| format!("Failed to parse JSON response from {}", url))?;
    let child = resp
        .into_iter()
        .nth(0)
        .unwrap()
        .data
        .children
        .into_iter()
        .nth(0)
        .unwrap();
    if let RedditSubmissionChild::Link { url } = child {
        Ok(Some(url))
    } else {
        Ok(None)
    }
}

#[derive(Deserialize)]
struct HnResponse {
    hits: Vec<HnHit>,
}

#[derive(Deserialize)]
pub(crate) struct HnHit {
    #[serde(rename = "objectID")]
    id: String,
    points: i64,
    created_at: String,
}

impl HnHit {
    pub fn discussion_url(&self) -> String {
        format!("https://news.ycombinator.com/item?id={}", self.id)
    }
}

impl Display for HnHit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let points = format!(
            "{} point{}",
            self.points,
            if self.points == 1 { "" } else { "s" }
        );
        write!(
            f,
            "{} | {} | {}",
            self.discussion_url(),
            points,
            self.created_at
        )
    }
}

/// Finds HN items for a given `url`.
pub(crate) async fn get_hn_discussions(
    url: Url,
    http_client: &reqwest::Client,
) -> Result<Vec<HnHit>> {
    let api_url = Url::parse_with_params(
        HN_SEARCH_URL,
        &[
            ("query", url.as_str()),
            ("numericFilters", "num_comments>0"),
            ("restrictSearchableAttributes", "url"),
        ],
    )?;
    let resp = http_client
        .get(api_url)
        .send()
        .await?
        .json::<HnResponse>()
        .await?;
    Ok(resp.hits)
}

#[derive(Deserialize)]
struct WaybackResponse {
    archived_snapshots: ArchivedSnapshots,
}

#[derive(Deserialize)]
struct ArchivedSnapshots {
    closest: Option<Closest>,
}

#[derive(Deserialize)]
struct Closest {
    url: String,
}

/// Returns Wayback Machine archive URL for a given `url`.
pub(crate) async fn get_wayback_url(
    url: String,
    time: Option<NaiveDateTime>,
    http_client: &reqwest::Client,
) -> Result<Option<String>> {
    // https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html
    // YYYYMMDDhhmmss
    let mut params = vec![("url", url)];
    if let Some(time) = time {
        let time = time.format("%Y%m%d%H%M%S").to_string();
        params.push(("timestamp", time));
    }
    let api_url = Url::parse_with_params(WAYBACK_URL, &params)?;
    let resp = http_client
        .get(api_url)
        .send()
        .await?
        .json::<WaybackResponse>()
        .await?;
    Ok(resp.archived_snapshots.closest.map(|c| c.url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reddit_hn_item_response_de() {
        let resp = r#"
        {
            "kind": "Listing",
            "data": {
            "children": [
                {
                    "kind": "t3",
                    "data": {
                        "url": "https://www.reddit.com/r/redditdev/comments/fcnkwq/documentation_for_rsubredditjson_api/"
                    }
                },
                {
                    "kind": "t1",
                    "data": {
                        "body": "foo"
                    }
                }
            ],
            "before": null
            }
        }
        "#;
        let resp: RedditSubmissionListing =
            serde_json::from_str(resp).expect("failed to deserialize payload");
        let expected = RedditSubmissionListing {
            data: RedditSubmissionListingData {
                children: vec![
                    RedditSubmissionChild::Link { url: "https://www.reddit.com/r/redditdev/comments/fcnkwq/documentation_for_rsubredditjson_api/".into() },
                    RedditSubmissionChild::Comment {}
                ],
            },
        };
        assert_eq!(resp, expected);
    }
}
