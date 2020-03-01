import * as googleTrends from "google-trends-api";

function main() {
  const yesterday = (() => {
    const d = new Date();
    d.setDate(d.getDate() - 1);
    return d;
  })();
  const today = new Date();

  googleTrends.dailyTrends(
    {
      startTime: yesterday,
      endTime: today,
      geo: "US"
    },
    (err: any, results: any) => {
      if (err) {
        console.error(err);
      } else {
        const data = JSON.parse(results);
        data.default.trendingSearchesDays[0].trendingSearches
          .slice(0, 5)
          .map((trend: any) => {
            console.log(trend.title);
          });
      }
    }
  );
}

main();
