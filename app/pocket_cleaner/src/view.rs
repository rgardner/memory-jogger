use actix_web::{self, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    trends::{self, TrendFinder},
};

#[derive(Deserialize)]
pub struct TrendsRequest {
    #[serde(default)]
    geo: trends::Geo,
}

#[derive(Serialize)]
struct TrendsResponse {
    trends: Vec<String>,
}

pub async fn trends_view(query: web::Query<TrendsRequest>) -> Result<impl Responder> {
    let trend_finder = TrendFinder::new();
    let trends = trend_finder.daily_trends(&query.geo).await?;

    Ok(HttpResponse::Ok().json(TrendsResponse {
        trends: trends.iter().map(|t| t.name()).collect(),
    }))
}
