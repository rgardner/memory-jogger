use std::convert::TryFrom;

use actix_web::{self, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::{
    error::{PocketCleanerError, Result},
    trends::{self, TrendFinder},
};

#[derive(Deserialize, Default, Clone, Debug)]
pub struct TrendsGeo(String);

impl TryFrom<TrendsGeo> for trends::Geo {
    type Error = PocketCleanerError;

    fn try_from(raw: TrendsGeo) -> std::result::Result<Self, Self::Error> {
        Self::new(raw.0)
    }
}

#[derive(Deserialize)]
pub struct TrendsRequest {
    #[serde(default)]
    geo: TrendsGeo,
}

#[derive(Serialize)]
struct TrendsResponse {
    trends: Vec<String>,
}

pub async fn trends_view(query: web::Query<TrendsRequest>) -> Result<impl Responder> {
    let trend_finder = TrendFinder::new();
    let geo = trends::Geo::try_from(query.geo.clone())?;
    let trends = trend_finder.daily_trends(&geo, 1 /*num_days*/).await?;

    Ok(HttpResponse::Ok().json(TrendsResponse {
        trends: trends.iter().map(|t| t.name()).collect(),
    }))
}
