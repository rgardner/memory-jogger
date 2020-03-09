//! A module for working with a user's [Pocket](https://getpocket.com) library.

use std::collections::HashMap;

use actix_web::{
    client::Client,
    http::{uri::Uri, PathAndQuery},
};
use serde::Deserialize;
use url::form_urlencoded;

use crate::error::{PocketCleanerError, Result};

pub struct PocketManager {
    consumer_key: String,
}

pub struct UserPocketManager {
    consumer_key: String,
    user_access_token: String,
}

impl PocketManager {
    pub fn new(consumer_key: String) -> Self {
        PocketManager { consumer_key }
    }

    pub fn for_user(&self, user_access_token: &str) -> UserPocketManager {
        UserPocketManager {
            consumer_key: self.consumer_key.clone(),
            user_access_token: user_access_token.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PocketItem {
    id: String,
    title: String,
    excerpt: String,
}

pub struct PocketPage {
    pub items: Vec<PocketItem>,
    pub since: i64,
}

impl UserPocketManager {
    pub async fn get_items(&self, keyword: &str) -> Result<Vec<PocketItem>> {
        let client = Client::default();
        let req = PocketRetrieveItemRequest {
            consumer_key: self.consumer_key.clone(),
            user_access_token: self.user_access_token.clone(),
            search: Some(keyword.into()),
        };
        let resp = send_pocket_retrieve_request(&client, &req).await?;
        Ok(resp.list.values().cloned().map(PocketItem::from).collect())
    }

    pub async fn get_items_paginated(
        &self,
        count: i32,
        offset: i32,
        since: Option<i64>,
    ) -> Result<PocketPage> {
        todo!()
    }
}

impl PocketItem {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    pub fn title(&self) -> String {
        self.title.clone()
    }
    pub fn excerpt(&self) -> String {
        self.excerpt.clone()
    }
}

impl From<RemotePocketItem> for PocketItem {
    fn from(remote: RemotePocketItem) -> Self {
        Self {
            id: remote.item_id.0,
            title: remote.resolved_title,
            excerpt: remote.excerpt,
        }
    }
}

struct PocketRetrieveItemRequest {
    consumer_key: String,
    user_access_token: String,
    search: Option<String>,
}

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
struct RemotePocketItemId(String);

#[derive(Deserialize, Debug)]
struct PocketRetrieveItemResponse {
    list: HashMap<RemotePocketItemId, RemotePocketItem>,
}

#[derive(Clone, Deserialize, Debug)]
struct RemotePocketItem {
    item_id: RemotePocketItemId,
    resolved_title: String,
    excerpt: String,
}

fn build_pocket_retrieve_url(req: &PocketRetrieveItemRequest) -> Result<Uri> {
    let mut query_builder = form_urlencoded::Serializer::new(String::new());
    query_builder.append_pair("consumer_key", &req.consumer_key);
    query_builder.append_pair("access_token", &req.user_access_token);
    if let Some(search) = &req.search {
        query_builder.append_pair("search", &search);
    }

    let encoded: String = query_builder.finish();

    let path_and_query: PathAndQuery = format!("/v3/get?{}", encoded).parse().unwrap();
    Ok(Uri::builder()
        .scheme("https")
        .authority("getpocket.com")
        .path_and_query(path_and_query)
        .build()
        .map_err(|e| PocketCleanerError::Logic(e.to_string()))?)
}

async fn send_pocket_retrieve_request(
    client: &Client,
    req: &PocketRetrieveItemRequest,
) -> Result<PocketRetrieveItemResponse> {
    let url = build_pocket_retrieve_url(req)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;
    let body = response
        .body()
        .await
        .map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;
    let body =
        std::str::from_utf8(&body).map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;

    let data: Result<PocketRetrieveItemResponse> =
        serde_json::from_str(body).map_err(|e| PocketCleanerError::Unknown(e.to_string()));

    match data {
        Ok(data) => Ok(data),
        Err(e) => {
            log::error!("failed to deserialize payload: {}", body);
            Err(e)
        }
    }
}
