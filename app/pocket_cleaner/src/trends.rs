//! A module for finding trending headlines and stories.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{PocketCleanerError, Result};

#[derive(Default)]
pub struct TrendFinder;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Geo(String);

impl Geo {
    pub fn new(raw: String) -> Result<Self> {
        if raw.is_empty() {
            return Err(PocketCleanerError::InvalidArgument(
                "geo must not be empty".into(),
            ));
        }

        Ok(Self(raw))
    }
}

impl Default for Geo {
    fn default() -> Self {
        Self("US".to_string())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Trend {
    name: String,
    explore_link: String,
}

impl fmt::Display for Trend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl TrendFinder {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn daily_trends(&self, geo: &Geo, num_days: u32) -> Result<Vec<Trend>> {
        let client = reqwest::Client::new();
        let mut trends = Vec::new();
        let mut trend_date: Option<String> = None;
        for _ in 0..num_days {
            let req = DailyTrendsRequest {
                geo: &geo,
                trend_date: trend_date.as_deref(),
            };
            let mut raw_trends = send_daily_trends_request(&client, &req).await?;
            trend_date = Some(raw_trends.default.end_date_for_next_request.clone());
            let day = raw_trends.default.trending_searches_days.remove(0);
            trends.extend(day.trending_searches.into_iter().map(Into::into))
        }

        Ok(trends)
    }
}

impl Trend {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns absolute URL to learn more about the trend.
    pub fn explore_link(&self) -> String {
        self.explore_link.clone()
    }
}

impl From<TrendingSearch> for Trend {
    fn from(search: TrendingSearch) -> Self {
        Self {
            name: search.title.query,
            explore_link: format!("https://trends.google.com{}", search.title.explore_link),
        }
    }
}

struct DailyTrendsRequest<'a> {
    pub geo: &'a Geo,
    pub trend_date: Option<&'a str>,
}

/// Top-level Google Trends Daily Trends API response.
#[derive(Deserialize, PartialEq, Debug)]
struct DailyTrendsResponse {
    default: DailyTrendsData,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct DailyTrendsData {
    trending_searches_days: Vec<TrendingSearchDay>,
    end_date_for_next_request: String,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct TrendingSearchDay {
    trending_searches: Vec<TrendingSearch>,
}

#[derive(Deserialize, PartialEq, Debug)]
struct TrendingSearch {
    title: TrendingSearchTitle,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
struct TrendingSearchTitle {
    /// Trend name.
    query: String,
    /// Relative URL to learn more about the trend.
    explore_link: String,
}

fn build_daily_trends_url(req: &DailyTrendsRequest) -> Result<reqwest::Url> {
    let mut params = vec![("geo", req.geo.0.as_str())];
    if let Some(trend_date) = req.trend_date {
        params.push(("ed", trend_date));
    }

    let url = reqwest::Url::parse_with_params(
        "https://trends.google.com/trends/api/dailytrends?",
        params,
    )
    .map_err(|e| PocketCleanerError::Logic(e.to_string()))?;
    Ok(url)
}

async fn send_daily_trends_request(
    client: &reqwest::Client,
    req: &DailyTrendsRequest<'_>,
) -> Result<DailyTrendsResponse> {
    let url = build_daily_trends_url(req)?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;
    let body = response
        .text()
        .await
        .map_err(|e| PocketCleanerError::Unknown(e.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    use url::Url;

    #[tokio::test]
    async fn test_geo_new_when_called_with_empty_string_returns_error() {
        let empty_geo = Geo::new("".into());
        assert!(empty_geo.is_err());
    }

    #[test]
    fn test_build_daily_trends_url_when_called_with_just_geo_returns_correct_url() {
        let geo = Geo::new("US".into()).unwrap();
        let req = DailyTrendsRequest {
            geo: &geo,
            trend_date: None,
        };

        let actual_url = build_daily_trends_url(&req).unwrap();

        let expected_url = "https://trends.google.com/trends/api/dailytrends?geo=US";
        let expected_url = Url::parse(expected_url).unwrap();
        assert_eq!(actual_url, expected_url);
    }

    #[test]
    fn test_build_daily_trends_url_when_called_with_end_data_returns_correct_url() {
        let geo = Geo::new("US".into()).unwrap();
        let req = DailyTrendsRequest {
            geo: &geo,
            trend_date: Some("20200313".into()),
        };

        let actual_url = build_daily_trends_url(&req).unwrap();

        let expected_url = "https://trends.google.com/trends/api/dailytrends?geo=US&ed=20200313";
        let expected_url = Url::parse(expected_url).unwrap();
        assert_eq!(actual_url, expected_url);
    }

    #[test]
    fn test_deserialize_trends_response() {
        let s = r#"{
            "default": {
                "trendingSearchesDays": [
                    {
                        "date": "20200314",
                        "formattedDate": "Saturday, March 14, 2020",
                        "trendingSearches": [
                            {
                                "title": {
                                    "query": "Coronavirus tips",
                                    "exploreLink": "/trends/explore?q=Coronavirus+tips&date=now+7-d&geo=US"
                                },
                                "formattedTraffic": "2M+",
                                "relatedQueries": [],
                                "image": {
                                    "newsUrl": "https://www.npr.org/2020/03/15/815549926/8-tips-to-make-working-from-home-work-for-you",
                                    "source": "NPR",
                                    "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcQOo9muBZru1De7RslRzgz3KTB0JLqIeR9Y3_1gv1HaDQF5NAxiI8vXVgtA_rcrKeTewqM0lyDR"
                                },
                                "articles": [
                                    {
                                        "title": "8 Tips To Make Working From Home, Work For You",
                                        "timeAgo": "4h ago",
                                        "source": "NPR",
                                        "image": {
                                            "newsUrl": "https://www.npr.org/2020/03/15/815549926/8-tips-to-make-working-from-home-work-for-you",
                                            "source": "NPR",
                                            "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcQOo9muBZru1De7RslRzgz3KTB0JLqIeR9Y3_1gv1HaDQF5NAxiI8vXVgtA_rcrKeTewqM0lyDR"
                                        },
                                        "url": "https://www.npr.org/2020/03/15/815549926/8-tips-to-make-working-from-home-work-for-you",
                                        "snippet": "Millions of people are trying to work from home because of coronavirus. Life Kit wants to help WFH work for you, especially if you&#39;re doing so for the first time."
                                    },
                                    {
                                        "title": "Home school teacher offers parents tips amid closings due to ...",
                                        "timeAgo": "15h ago",
                                        "source": "FOX 5 Atlanta",
                                        "image": {
                                            "newsUrl": "https://www.fox5atlanta.com/news/home-school-teacher-offers-parents-tips-amid-closings-due-to-coronavirus",
                                            "source": "FOX 5 Atlanta",
                                            "imageUrl": "https://t3.gstatic.com/images?q=tbn:ANd9GcRhydo2GMnZx78Ofra9idgAMt9aR1q6aJTDaDz1Zt-sTEGjagXZH5-FTnI1kGYPpikgIqW0fSd_"
                                        },
                                        "url": "https://www.fox5atlanta.com/news/home-school-teacher-offers-parents-tips-amid-closings-due-to-coronavirus",
                                        "snippet": "Schools across the country will be closing beginning Monday as fears of the spread of the coronavirus continue. This will create a unique situation where&nbsp;..."
                                    },
                                    {
                                        "title": "Your coronavirus emergency kit: Preparation, symptoms, tips",
                                        "timeAgo": "6h ago",
                                        "source": "Aljazeera.com",
                                        "image": {
                                            "newsUrl": "https://www.aljazeera.com/news/2020/03/coronavirus-emergency-kit-preparation-symptoms-tips-200314103304717.html",
                                            "source": "Aljazeera.com",
                                            "imageUrl": "https://t3.gstatic.com/images?q=tbn:ANd9GcSChU3cpVibmIJsLc7NaKkkot5zOxBmEhzIlw4JhCgXgCRtyj90v_RP9Oe_6EcCapargjj9rCOO"
                                        },
                                        "url": "https://www.aljazeera.com/news/2020/03/coronavirus-emergency-kit-preparation-symptoms-tips-200314103304717.html",
                                        "snippet": "How to prepare or deal with COVID-19 as well as survive a virus-related lockdown."
                                    },
                                    {
                                        "title": "Working from home because of coronavirus? Be careful what you ...",
                                        "timeAgo": "5h ago",
                                        "source": "USA TODAY",
                                        "image": {
                                            "newsUrl": "https://www.usatoday.com/story/tech/2020/03/15/coronavirus-cyber-safety-tips-working-home/5034081002/",
                                            "source": "USA TODAY",
                                            "imageUrl": "https://t3.gstatic.com/images?q=tbn:ANd9GcQQ3PRxORS9J_tIR9BTZUpCDOiNTMDsgADDb8eLrVe0YnBt-Z5fKzOatWqcV8e6wjAVNkwdOf7E"
                                        },
                                        "url": "https://www.usatoday.com/story/tech/2020/03/15/coronavirus-cyber-safety-tips-working-home/5034081002/",
                                        "snippet": "Tips from cybersecurity experts to keep you safe and your computer (and boss) happy while you&#39;re working from home during the coronavirus outbreak."
                                    }
                                ],
                                "shareUrl": "https://trends.google.com/trends/trendingsearches/daily?geo=US&tt=Coronavirus+tips#Coronavirus%20tips"
                            },
                            {
                                "title": {
                                    "query": "Pi",
                                    "exploreLink": "/trends/explore?q=Pi&date=now+7-d&geo=US"
                                },
                                "formattedTraffic": "500K+",
                                "relatedQueries": [
                                    {
                                        "query": "pi day",
                                        "exploreLink": "/trends/explore?q=pi+day&date=now+7-d&geo=US"
                                    },
                                    {
                                        "query": "pi day 2020",
                                        "exploreLink": "/trends/explore?q=pi+day+2020&date=now+7-d&geo=US"
                                    },
                                    {
                                        "query": "pi day deals",
                                        "exploreLink": "/trends/explore?q=pi+day+deals&date=now+7-d&geo=US"
                                    }
                                ],
                                "image": {
                                    "newsUrl": "https://www.timesfreepress.com/news/life/entertainment/story/2020/mar/14/pi-day-specials-region/518189/",
                                    "source": "Chattanooga Times Free Press",
                                    "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcQjP1Lf71vbIRdACcOx2pILmRmO2KFYQoNI35w6XZwjrQpyvbEF5w8UcHUB5jv2oaNkO6FWPxSS"
                                },
                                "articles": [
                                    {
                                        "title": "Check out these Pi Day specials in the Chattanooga region",
                                        "timeAgo": "22h ago",
                                        "source": "Chattanooga Times Free Press",
                                        "image": {
                                            "newsUrl": "https://www.timesfreepress.com/news/life/entertainment/story/2020/mar/14/pi-day-specials-region/518189/",
                                            "source": "Chattanooga Times Free Press",
                                            "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcQjP1Lf71vbIRdACcOx2pILmRmO2KFYQoNI35w6XZwjrQpyvbEF5w8UcHUB5jv2oaNkO6FWPxSS"
                                        },
                                        "url": "https://www.timesfreepress.com/news/life/entertainment/story/2020/mar/14/pi-day-specials-region/518189/",
                                        "snippet": "What could be better than a pie on Pi Day, March 14, or numerically 03/14? How about paying $3.14 for a pizza, or apple pie?"
                                    },
                                    {
                                        "title": "16 Best Pi Day Deals to Take Advantage of on 3/14",
                                        "timeAgo": "1d ago",
                                        "source": "GoodHousekeeping.com",
                                        "image": {
                                            "newsUrl": "https://www.goodhousekeeping.com/life/money/a31404597/pi-day-deals-2020/",
                                            "source": "GoodHousekeeping.com",
                                            "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcREfQBEIoXUbGqneiR0NJ_NrxSqP13MVluDkiQROIiClEcigbOaoptuqWiKPRoQofA_8Y0_jV3F"
                                        },
                                        "url": "https://www.goodhousekeeping.com/life/money/a31404597/pi-day-deals-2020/",
                                        "snippet": "This year, Pi Day takes place on Saturday, March 14. Make sure to celebrate with these incredible Pi Day deals on all types of pies, both savory and sweet."
                                    },
                                    {
                                        "title": "LIST | Pi Day Deals Around Maryland",
                                        "timeAgo": "23h ago",
                                        "source": "CBS Baltimore",
                                        "url": "https://baltimore.cbslocal.com/2020/03/14/list-pi-day-deals-around-maryland/",
                                        "snippet": "(CNN) — If you like to save money, then you&#39;ll love today — It&#39;s Pi Day. Pi, or 3.14159265 and so on, is the ratio of the circumference of a circle to its diameter."
                                    },
                                    {
                                        "title": "Blaze Pizza reschedules Pi Day deal, extends through end of the year",
                                        "timeAgo": "23h ago",
                                        "source": "KLAS - 8 News Now",
                                        "image": {
                                            "newsUrl": "https://www.8newsnow.com/news/local-news/blaze-pizza-reschedules-pi-day-deal-extends-through-end-of-the-year/",
                                            "source": "KLAS - 8 News Now",
                                            "imageUrl": "https://t3.gstatic.com/images?q=tbn:ANd9GcTZkD9B1P15NFdvF9DWss2dAp2zJghqAPopwSHbgIOuOyMlX5-4CXsWYJtx5Z5YlNMlViqzuHXV"
                                        },
                                        "url": "https://www.8newsnow.com/news/local-news/blaze-pizza-reschedules-pi-day-deal-extends-through-end-of-the-year/",
                                        "snippet": "LAS VEGAS (CNN/KLAS) — Saturday is the 14th day of the third month, which means March 14 is Pi Day! The day is meant to celebrate the mathematical&nbsp;..."
                                    },
                                    {
                                        "title": "National Pi Day 2020 Deals: Discounts at Blaze Pizza, Papa Johns ...",
                                        "timeAgo": "23h ago",
                                        "source": "Newsweek",
                                        "image": {
                                            "newsUrl": "https://www.newsweek.com/national-pi-day-2020-deals-1492130",
                                            "source": "Newsweek",
                                            "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcTLdUggFe2CucSgpWsWInjhCvcmFaq4OZ6ncMNs0qw5Lo69IiPUID4sxWYRYF4W4F2z3nU3YlqY"
                                        },
                                        "url": "https://www.newsweek.com/national-pi-day-2020-deals-1492130",
                                        "snippet": "Today, March 14 (3/14), is National Pi Day, a day that celebrates the mathematical symbol we all came to know and love in school. Also known as π, the Pi is the&nbsp;..."
                                    },
                                    {
                                        "title": "National Pi Day 2020 Deals &amp; Freebies",
                                        "timeAgo": "1d ago",
                                        "source": "Heavy.com",
                                        "image": {
                                            "newsUrl": "https://heavy.com/entertainment/2020/03/pi-day-2020-deals-freebies-specials/",
                                            "source": "Heavy.com",
                                            "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcRbR1g-6B768Ke-JgbEEIxRPar9HLlkCOuH-5SjSyprFzTltL8fPB0cNa4UeFkrrPsMwh7wYXXe"
                                        },
                                        "url": "https://heavy.com/entertainment/2020/03/pi-day-2020-deals-freebies-specials/",
                                        "snippet": "In honor of National Pi Day 2020 on March 14, many restaurants and fast-food chains are offering deals and discounts to customers."
                                    },
                                    {
                                        "title": "Pi Day 2020 deals: Free pizza, free pie, and all the other top deals",
                                        "timeAgo": "1d ago",
                                        "source": "BGR",
                                        "image": {
                                            "newsUrl": "https://bgr.com/2020/03/14/pi-day-2020-deals-free-pizza-free-pie-best-deals/",
                                            "source": "BGR",
                                            "imageUrl": "https://t2.gstatic.com/images?q=tbn:ANd9GcRGV5QB5XLPw4WPZGr22EVnJHKPhyFRqUgFKowt0NbG1Fsl0Zw0cbubXGLCrjAZvVmcEdYzsNtH"
                                        },
                                        "url": "https://bgr.com/2020/03/14/pi-day-2020-deals-free-pizza-free-pie-best-deals/",
                                        "snippet": "Pi Day 2020 is today, and there are tons of deals available from big restaurants and retailers across the country. All the best Pi Day deals that are available out&nbsp;..."
                                    }
                                ],
                                "shareUrl": "https://trends.google.com/trends/trendingsearches/daily?geo=US&tt=Pi#Pi"
                            }
                        ]
                    }
                ],
                "endDateForNextRequest": "20200313",
                "rssFeedPageUrl": "https://trends.google.com/trends/trendingsearches/daily/rss?geo=US"
            }
        }"#;
        let resp: DailyTrendsResponse =
            serde_json::from_str(s).expect("failed to deserialize payload");
        assert_eq!(
            resp,
            DailyTrendsResponse {
                default: DailyTrendsData {
                    trending_searches_days: vec![TrendingSearchDay {
                        trending_searches: vec![
                            TrendingSearch {
                                title: TrendingSearchTitle {
                                    query: "Coronavirus tips".into(),
                                    explore_link:
                                        "/trends/explore?q=Coronavirus+tips&date=now+7-d&geo=US"
                                            .into(),
                                }
                            },
                            TrendingSearch {
                                title: TrendingSearchTitle {
                                    query: "Pi".into(),
                                    explore_link: "/trends/explore?q=Pi&date=now+7-d&geo=US".into(),
                                }
                            }
                        ],
                    }],
                    end_date_for_next_request: "20200313".into(),
                },
            }
        );
    }

    #[test]
    fn test_trend_from_trending_search() {
        let trending_search = TrendingSearch {
            title: TrendingSearchTitle {
                query: "FakeName".into(),
                explore_link: "/fake_link".into(),
            },
        };
        let actual_trend = Trend::from(trending_search);
        assert_eq!(
            actual_trend,
            Trend {
                name: "FakeName".into(),
                explore_link: "https://trends.google.com/fake_link".into(),
            }
        );
    }
}
