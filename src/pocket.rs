//! A module for working with a user's [Pocket](https://getpocket.com) library.

use std::{collections::HashMap, convert::TryFrom, fmt};

use chrono::NaiveDateTime;
use serde::Deserialize;

use crate::error::{Error, Result};

static REDIRECT_URI: &str = "memory_jogger:finishauth";

pub struct PocketManager<'a> {
    consumer_key: String,
    client: &'a reqwest::Client,
}

pub struct UserPocketManager<'a> {
    consumer_key: String,
    user_access_token: String,
    client: &'a reqwest::Client,
}

impl<'a> PocketManager<'a> {
    pub fn new(consumer_key: String, client: &'a reqwest::Client) -> Self {
        Self {
            consumer_key,
            client,
        }
    }

    /// Returns authorization URL and request token.
    pub async fn get_auth_url(&self) -> Result<(reqwest::Url, String)> {
        let url = reqwest::Url::parse_with_params(
            "https://getpocket.com/v3/oauth/request",
            &[
                ("consumer_key", self.consumer_key.as_str()),
                ("redirect_uri", REDIRECT_URI),
            ],
        )?;
        let resp = self.client.post(url).send().await?.error_for_status()?;
        let text = resp.text().await?;
        let request_token = text
            .split('=')
            .nth(1)
            .ok_or_else(|| Error::Unknown("Invalid response from Pocket".into()))?;

        let auth_url = reqwest::Url::parse_with_params(
            "https://getpocket.com/auth/authorize",
            &[
                ("request_token", request_token),
                ("redirect_uri", REDIRECT_URI),
            ],
        )?;

        Ok((auth_url, request_token.into()))
    }

    pub async fn authorize(&self, request_token: &str) -> Result<String> {
        let url = reqwest::Url::parse_with_params(
            "https://getpocket.com/v3/oauth/authorize",
            &[
                ("consumer_key", self.consumer_key.as_str()),
                ("code", request_token),
            ],
        )?;
        let resp = self.client.post(url).send().await?.error_for_status()?;
        let text = resp.text().await?;
        let access_token = text
            .split('&')
            .next()
            .and_then(|access_token_query_param| access_token_query_param.split('=').nth(1))
            .ok_or_else(|| Error::Unknown("Invalid response from Pocket".into()))?;

        Ok(access_token.into())
    }

    pub fn for_user(&self, user_access_token: String) -> UserPocketManager {
        UserPocketManager {
            consumer_key: self.consumer_key.clone(),
            user_access_token,
            client: &self.client,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PocketItemStatus {
    Unread,
    Archived,
    Deleted,
}

impl fmt::Display for PocketItemStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Unread => write!(f, "unread"),
            Self::Archived => write!(f, "archived"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

impl From<RemotePocketItemStatus> for PocketItemStatus {
    fn from(status: RemotePocketItemStatus) -> Self {
        match status {
            RemotePocketItemStatus::Unread => Self::Unread,
            RemotePocketItemStatus::Archived => Self::Archived,
            RemotePocketItemStatus::Deleted => Self::Deleted,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PocketItem {
    Unread {
        id: String,
        title: String,
        excerpt: String,
        url: String,
        time_added: NaiveDateTime,
    },
    ArchivedOrDeleted {
        id: String,
        status: PocketItemStatus,
    },
}

pub struct PocketPage {
    pub items: Vec<PocketItem>,
    pub since: i64,
}

#[derive(Default)]
pub struct PocketRetrieveQuery<'a> {
    pub state: Option<PocketRetrieveItemState>,
    pub search: Option<&'a str>,
    pub count: Option<u32>,
    pub offset: Option<u32>,
    pub since: Option<i64>,
}

impl<'a> UserPocketManager<'a> {
    pub async fn retrieve(&self, query: &PocketRetrieveQuery<'_>) -> Result<PocketPage> {
        let req = PocketRetrieveItemRequest {
            consumer_key: &self.consumer_key,
            user_access_token: &self.user_access_token,
            state: query.state,
            search: query.search.as_deref(),
            since: query.since,
            count: query.count,
            offset: query.offset,
        };
        let resp = send_pocket_retrieve_request(&self.client, &req).await?;
        let items = match resp.list {
            PocketRetrieveItemList::Map(items) => items
                .values()
                .cloned()
                .map(PocketItem::try_from)
                .collect::<Result<Vec<_>>>()?,
            PocketRetrieveItemList::List(_) => Vec::new(),
        };
        Ok(PocketPage {
            items,
            since: resp.since,
        })
    }
}

impl TryFrom<RemotePocketItem> for PocketItem {
    type Error = Error;

    fn try_from(remote: RemotePocketItem) -> std::result::Result<Self, Self::Error> {
        if remote.status == RemotePocketItemStatus::Archived
            || remote.status == RemotePocketItemStatus::Deleted
        {
            return Ok(Self::ArchivedOrDeleted {
                id: remote.item_id.0,
                status: remote.status.into(),
            });
        }

        let str_nonempty = |s: &String| !s.is_empty();
        let best_url = remote
            .resolved_url
            .filter(str_nonempty)
            .or(remote.given_url);

        let best_title = remote
            .resolved_title
            .filter(str_nonempty)
            .or(remote.given_title)
            .filter(str_nonempty)
            .or_else(|| best_url.clone())
            .filter(str_nonempty)
            .unwrap_or_default();

        let time_added = remote
            .time_added
            .ok_or_else(|| Error::Unknown("No time_added in Pocket item".into()))?
            .parse::<i64>()
            .map_err(|e| Error::Unknown(format!("Cannot parse time_added from Pocket: {}", e)))?;
        Ok(Self::Unread {
            id: remote.item_id.0,
            title: best_title,
            excerpt: remote.excerpt.unwrap_or_default(),
            url: best_url.unwrap_or_default(),
            time_added: NaiveDateTime::from_timestamp(time_added, 0 /*nsecs*/),
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PocketRetrieveItemState {
    Unread,
    Archive,
    All,
}

impl fmt::Display for PocketRetrieveItemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Unread => write!(f, "unread"),
            Self::Archive => write!(f, "archive"),
            Self::All => write!(f, "all"),
        }
    }
}

struct PocketRetrieveItemRequest<'a> {
    consumer_key: &'a str,
    user_access_token: &'a str,
    state: Option<PocketRetrieveItemState>,
    search: Option<&'a str>,
    since: Option<i64>,
    count: Option<u32>,
    offset: Option<u32>,
}

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
struct RemotePocketItemId(String);

#[derive(Deserialize, Debug, PartialEq)]
struct PocketRetrieveItemResponse {
    list: PocketRetrieveItemList,
    since: i64,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum PocketRetrieveItemList {
    Map(HashMap<RemotePocketItemId, RemotePocketItem>),
    List(Vec<()>),
}

#[derive(Copy, Clone, Deserialize, Debug, PartialEq)]
#[serde(try_from = "String")]
#[repr(u8)]
enum RemotePocketItemStatus {
    Unread = 0,
    Archived = 1,
    Deleted = 2,
}

impl TryFrom<String> for RemotePocketItemStatus {
    type Error = Error;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        match &s[..] {
            "0" => Ok(Self::Unread),
            "1" => Ok(Self::Archived),
            "2" => Ok(Self::Deleted),
            v => Err(Self::Error::InvalidArgument(format!(
                "Unknown Remote Pocket Item Status: {}",
                v
            ))),
        }
    }
}

#[derive(Clone, Deserialize, PartialEq, Debug)]
struct RemotePocketItem {
    pub item_id: RemotePocketItemId,
    pub given_url: Option<String>,
    /// Final URL after resolving URL shorteners and stripping some query
    /// parameters. May be empty.
    pub resolved_url: Option<String>,
    pub given_title: Option<String>,
    pub resolved_title: Option<String>,
    pub status: RemotePocketItemStatus,
    pub excerpt: Option<String>,
    pub time_added: Option<String>,
}

fn build_pocket_retrieve_url(req: &PocketRetrieveItemRequest) -> Result<reqwest::Url> {
    let mut params = vec![
        ("consumer_key", req.consumer_key.to_string()),
        ("access_token", req.user_access_token.to_string()),
    ];
    if let Some(state) = &req.state {
        params.push(("state", state.to_string()));
    }
    if let Some(search) = &req.search {
        params.push(("search", search.to_string()));
    }
    if let Some(since) = &req.since {
        params.push(("since", since.to_string()));
    }
    if let Some(count) = &req.count {
        params.push(("count", count.to_string()));
    }
    if let Some(offset) = &req.offset {
        params.push(("offset", offset.to_string()));
    }

    let url = reqwest::Url::parse_with_params("https://getpocket.com/v3/get", params)?;
    Ok(url)
}

async fn send_pocket_retrieve_request(
    client: &reqwest::Client,
    req: &PocketRetrieveItemRequest<'_>,
) -> Result<PocketRetrieveItemResponse> {
    let url = build_pocket_retrieve_url(req)?;

    let mut num_attempts = 0;
    let response = loop {
        if num_attempts == 3 {
            return Err(Error::Unknown(format!(
                "failed to connect to or receive a response from Pocket after {} attempts",
                num_attempts
            )));
        }
        let response = client
            .get(url.clone())
            .send()
            .await
            .and_then(|e| e.error_for_status());
        num_attempts += 1;
        match response {
            Ok(resp) => break resp,
            Err(e) if e.is_timeout() => continue,
            Err(e) => return Err(e.into()),
        }
    };

    let data: PocketRetrieveItemResponse = response.json().await?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    use reqwest::Url;

    #[test]
    fn test_build_pocket_retrieve_url_when_called_minimal_returns_correct_url() {
        let req = PocketRetrieveItemRequest {
            consumer_key: "fake_consumer_key",
            user_access_token: "fake_user_access_token",
            count: None,
            offset: None,
            state: None,
            search: None,
            since: None,
        };

        let actual_url = build_pocket_retrieve_url(&req).unwrap();

        let expected_url = "https://getpocket.com/v3/get?consumer_key=fake_consumer_key&access_token=fake_user_access_token";
        let expected_url = Url::parse(expected_url).unwrap();
        assert_eq!(actual_url, expected_url);
    }

    #[test]
    fn test_build_pocket_retrieve_url_when_called_sync_params_returns_correct_url() {
        let req = PocketRetrieveItemRequest {
            consumer_key: "fake_consumer_key",
            user_access_token: "fake_user_access_token",
            count: Some(5),
            offset: Some(10),
            state: Some(PocketRetrieveItemState::All),
            search: None,
            since: None,
        };

        let actual_url = build_pocket_retrieve_url(&req).unwrap();

        let expected_url = "https://getpocket.com/v3/get?consumer_key=fake_consumer_key&access_token=fake_user_access_token&state=all&count=5&offset=10";
        let expected_url = Url::parse(expected_url).unwrap();
        assert_eq!(actual_url, expected_url);
    }

    #[test]
    fn test_deserialize_pocket_page_with_multiple_items() {
        let s = r#"
        {
            "status": 1,
            "complete": 1,
            "list": {
                "64966083": {
                    "item_id": "64966083",
                    "resolved_id": "64966083",
                    "given_url": "http://www.inc.com/magazine/20110201/how-great-entrepreneurs-think.html",
                    "given_title": "How Great Entrepreneurs Think | Inc.com",
                    "favorite": "0",
                    "status": "0",
                    "time_added": "1363453123",
                    "time_updated": "1363484394",
                    "time_read": "0",
                    "time_favorited": "0",
                    "sort_id": 0,
                    "resolved_title": "How Great Entrepreneurs Think",
                    "resolved_url": "https://www.inc.com/magazine/20110201/how-great-entrepreneurs-think.html",
                    "excerpt": "MockExcerpt1",
                    "is_article": "1",
                    "is_index": "0",
                    "has_video": "0",
                    "has_image": "1",
                    "word_count": "2879",
                    "lang": "en",
                    "time_to_read": 13,
                    "top_image_url": "https://www.incimages.com/uploaded_files/image/970x450/EntrepreneursThink_Pan_6964.jpg",
                    "domain_metadata": {
                        "name": "Inc. Magazine",
                        "logo": "https://logo.clearbit.com/inc.com?size=800",
                        "greyscale_logo": "https://logo.clearbit.com/inc.com?size=800&greyscale=true"
                    },
                    "listen_duration_estimate": 1114
                },
                "262512228": {
                    "item_id": "262512228",
                    "resolved_id": "260475629",
                    "given_url": "http://codenerdz.com/blog/2012/12/03/think-of-selling-on-ebay-using-paypal-think-again/?utm_source=hackernewsletter&utm_medium=email",
                    "given_title": "Thinking of selling on eBay with PayPal? Think again! - CodeNerdz",
                    "favorite": "0",
                    "status": "1",
                    "time_added": "1363453110",
                    "time_updated": "1363453110",
                    "time_read": "0",
                    "time_favorited": "0",
                    "sort_id": 1,
                    "resolved_title": "",
                    "resolved_url": "http://codenerdz.com/blog/2012/12/03/think-of-selling-on-ebay-using-paypal-think-again/",
                    "excerpt": "",
                    "is_article": "0",
                    "is_index": "0",
                    "has_video": "0",
                    "has_image": "0",
                    "word_count": "0",
                    "lang": "en",
                    "listen_duration_estimate": 0
                }
            },
            "error": null,
            "search_meta": {
                "search_type": "normal"
            },
            "since": 1583723171
        }
        "#;
        let resp: PocketRetrieveItemResponse =
            serde_json::from_str(s).expect("failed to deserialize payload");
        assert_eq!(
            resp,
            PocketRetrieveItemResponse {
                list: PocketRetrieveItemList::Map([(RemotePocketItemId("64966083".into()), RemotePocketItem {
                    item_id: RemotePocketItemId("64966083".into()),
                    given_url: Some("http://www.inc.com/magazine/20110201/how-great-entrepreneurs-think.html".into()),
                    resolved_url: Some("https://www.inc.com/magazine/20110201/how-great-entrepreneurs-think.html".into()),
                    given_title: Some("How Great Entrepreneurs Think | Inc.com".into()),
                    resolved_title: Some("How Great Entrepreneurs Think".into()),
                    status: RemotePocketItemStatus::Unread,
                    excerpt: Some("MockExcerpt1".into()),
                    time_added: Some("1363453123".into()),
                }), (RemotePocketItemId("262512228".into()), RemotePocketItem {
                    item_id: RemotePocketItemId("262512228".into()),
                    given_url: Some("http://codenerdz.com/blog/2012/12/03/think-of-selling-on-ebay-using-paypal-think-again/?utm_source=hackernewsletter&utm_medium=email".into()),
                    resolved_url: Some("http://codenerdz.com/blog/2012/12/03/think-of-selling-on-ebay-using-paypal-think-again/".into()),
                    given_title: Some("Thinking of selling on eBay with PayPal? Think again! - CodeNerdz".into()),
                    resolved_title: Some("".into()),
                    status: RemotePocketItemStatus::Archived,
                    excerpt: Some("".into()),
                    time_added: Some("1363453110".into()),
                })].iter().cloned().collect::<HashMap<RemotePocketItemId, RemotePocketItem>>()),
                since: 1583723171,
            }
        );
    }

    #[test]
    fn test_deserialize_pocket_page_with_deleted_item() {
        let s = r#"
        {
            "status": 1,
            "complete": 0,
            "list": {
                "2929045771": {
                    "item_id": "2929045771",
                    "status": "2",
                    "listen_duration_estimate": 0
                }
            },
            "error": null,
            "search_meta": {
                "search_type": "normal"
            },
            "since": 1585393208
        }
        "#;
        let resp: PocketRetrieveItemResponse =
            serde_json::from_str(s).expect("failed to deserialize payload");
        assert_eq!(
            resp,
            PocketRetrieveItemResponse {
                list: PocketRetrieveItemList::Map(
                    [(
                        RemotePocketItemId("2929045771".into()),
                        RemotePocketItem {
                            item_id: RemotePocketItemId("2929045771".into()),
                            status: RemotePocketItemStatus::Deleted,
                            given_url: None,
                            resolved_url: None,
                            given_title: None,
                            resolved_title: None,
                            excerpt: None,
                            time_added: None,
                        }
                    )]
                    .iter()
                    .cloned()
                    .collect::<HashMap<RemotePocketItemId, RemotePocketItem>>()
                ),
                since: 1585393208,
            }
        );
    }

    #[test]
    fn test_deserialize_last_pocket_page() {
        let s = r#"{"status":2,"complete":1,"list":[],"error":null,"search_meta":{"search_type":"normal"},"since":1583763395}"#;
        let resp: PocketRetrieveItemResponse =
            serde_json::from_str(s).expect("failed to deserialize payload");
        assert_eq!(
            resp,
            PocketRetrieveItemResponse {
                list: PocketRetrieveItemList::List(vec![]),
                since: 1583763395,
            }
        );
    }

    #[test]
    fn test_deserialize_remote_pocket_item_status_unread() {
        let s = r#""0""#;
        let status: RemotePocketItemStatus =
            serde_json::from_str(s).expect("failed to deserialize payload");
        assert_eq!(status, RemotePocketItemStatus::Unread);
    }

    #[test]
    fn test_deserialize_remote_pocket_item_status_archived() {
        let s = r#""1""#;
        let status: RemotePocketItemStatus =
            serde_json::from_str(s).expect("failed to deserialize payload");
        assert_eq!(status, RemotePocketItemStatus::Archived);
    }

    #[test]
    fn test_deserialize_remote_pocket_item_status_deleted() {
        let s = r#""2""#;
        let status: RemotePocketItemStatus =
            serde_json::from_str(s).expect("failed to deserialize payload");
        assert_eq!(status, RemotePocketItemStatus::Deleted);
    }
}
