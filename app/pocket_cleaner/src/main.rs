//! Surfaces items from your [Pocket](https://getpocket.com) library based on
//! trending headlines.

use crate::trends::{Geo, TrendFinder};

mod trends;

#[actix_rt::main]
async fn main() {
    let trend_finder = TrendFinder::new();
    let trends = trend_finder.daily_trends(&Geo("US".into())).await.unwrap();
    for (i, trend) in trends.iter().enumerate() {
        println!("{}. {}", i, trend.name());
    }
}
