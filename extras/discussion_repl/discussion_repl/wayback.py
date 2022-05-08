"""API client for Wayback Machine.

https://archive.org/help/wayback_api.php
"""

import datetime

import requests

BASE_URL = "http://archive.org/wayback/available"


def get_snapshot(url: str, timestamp: datetime.datetime) -> str | None:
    """Returns URL to Wayback Machine snapshot."""
    ts_str = timestamp.isoformat().replace("T", "").replace("-", "").replace(":", "")
    params = {
        "url": url,
        "timestamp": ts_str,
    }
    resp = requests.get(BASE_URL, params=params)
    resp.raise_for_status()
    data = resp.json()
    return parse_url_from_snapshot(data)


def parse_url_from_snapshot(data: dict) -> str | None:
    """Parses URL from Wayback Machine snapshot API response."""
    if data["archived_snapshots"]:
        return data["archived_snapshots"]["closest"]["url"]

    return None
