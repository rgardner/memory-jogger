"""Hacker News API wrapper."""

from __future__ import annotations

import dataclasses
import datetime

import requests

HN_SEARCH_URL = "https://hn.algolia.com/api/v1/search"


@dataclasses.dataclass
class HNItem:
    """Hacker News (HN) item."""

    id: str
    points: int
    created_at: datetime.datetime

    @property
    def discussion_url(self) -> str:
        """Returns URL to Hacker News discussion."""
        return f"https://news.ycombinator.com/item?id={self.id}"

    def __str__(self) -> str:
        points = f"{self.points} point" + ("s" if self.points != 1 else "")
        return f"{self.discussion_url} | {points} | {self.created_at.isoformat()}"

    def to_json_dict(self) -> dict:
        """Returns JSON representation of the item."""
        return {
            "objectID": self.id,
            "points": self.points,
            "created_at_i": self.created_at.timestamp(),
        }

    @staticmethod
    def from_json(json: dict) -> HNItem:
        """Creates HNItem from JSON."""
        return HNItem(
            id=json["objectID"],
            points=json["points"],
            created_at=datetime.datetime.fromtimestamp(json["created_at_i"]),
        )


def find_and_display_discussions_non_hn(
    url: str, exclude_id: str | None = None
) -> None:
    """Finds HackerNews discussions for the given URL.

    :raises requests.RequestException: HN API request failed
    """
    params = {
        "query": url,
        "numericFilters": "num_comments>0",
        "restrictSearchableAttributes": "url",
    }
    resp = requests.get(HN_SEARCH_URL, params=params)
    resp.raise_for_status()
    print("\n".join(format_discussions(resp.json(), exclude_id)))


def format_discussions(data: dict, exclude_id: str | None = None) -> list[str]:
    """Returns formatted discussions."""
    items = [HNItem.from_json(item) for item in data["hits"]]
    items = [item for item in items if item.id != exclude_id]
    items = sorted(items, key=lambda item: item.points, reverse=True)
    return [str(item) for item in items]
