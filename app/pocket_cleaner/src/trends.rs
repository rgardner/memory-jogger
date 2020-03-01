//! A module for finding trending headlines and stories.

use actix_web::{
    client::Client,
    http::{uri::Uri, PathAndQuery},
};
use serde::Deserialize;
use url::form_urlencoded;

use crate::error::{PocketCleanerError, Result};

pub struct TrendFinder;

#[derive(Clone, Debug)]
pub struct Geo(pub String);

pub struct Trend {
    name: String,
}

impl TrendFinder {
    pub fn new() -> Self {
        TrendFinder {}
    }

    pub async fn daily_trends(&self, geo: &Geo) -> Result<Vec<Trend>> {
        let client = Client::default();
        let req = DailyTrendsRequest::new(geo.clone());
        let mut raw_trends = send_daily_trends_request(&client, &req).await?;
        let day = raw_trends.default.trending_searches_days.remove(0);
        Ok(day.trending_searches.into_iter().map(Into::into).collect())
    }
}

impl Trend {
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl From<TrendingSearch> for Trend {
    fn from(search: TrendingSearch) -> Self {
        Self {
            name: search.title.query,
        }
    }
}

struct DailyTrendsRequest {
    geo: Geo,
}

impl DailyTrendsRequest {
    fn new(geo: Geo) -> Self {
        DailyTrendsRequest { geo }
    }
}

/// Top-level Google Trends Daily Trends API response.
#[derive(Deserialize, Debug)]
struct DailyTrendsResponse {
    default: DailyTrendsData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DailyTrendsData {
    trending_searches_days: Vec<TrendingSearchDay>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TrendingSearchDay {
    trending_searches: Vec<TrendingSearch>,
}

#[derive(Deserialize, Debug)]
struct TrendingSearch {
    title: TrendingSearchTitle,
}

#[derive(Deserialize, Debug)]
struct TrendingSearchTitle {
    /// Trending search query.
    query: String,
}

fn build_daily_trends_url(req: &DailyTrendsRequest) -> Result<Uri> {
    let mut query_builder = form_urlencoded::Serializer::new(String::new());
    query_builder.append_pair("geo", &req.geo.0);
    let encoded: String = query_builder.finish();

    let path_and_query: PathAndQuery = format!("/trends/api/dailytrends?{}", encoded)
        .parse()
        .unwrap();
    Ok(Uri::builder()
        .scheme("https")
        .authority("trends.google.com")
        .path_and_query(path_and_query)
        .build()
        .map_err(|e| PocketCleanerError::Logic(e.to_string()))?)
}

async fn send_daily_trends_request(
    client: &Client,
    req: &DailyTrendsRequest,
) -> Result<DailyTrendsResponse> {
    let url = build_daily_trends_url(req)?;
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

    // For some reason, Google Trends prepends 5 characters at the start of the
    // response that makes this invalid JSON, specifically: ")]}',"
    let data: Result<DailyTrendsResponse> =
        serde_json::from_str(&body[5..]).map_err(|e| PocketCleanerError::Unknown(e.to_string()));

    match data {
        Ok(data) => Ok(data),
        Err(e) => {
            log::error!("failed to deserialize payload: {}", body);
            Err(e)
        }
    }
}
