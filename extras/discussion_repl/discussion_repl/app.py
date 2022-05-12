"""Finds Hacker News discussions for a given article.

TODO:
- show nearest discussion first
- add Reddit support
- show Pocket URL and search fallback (see main.rs)

NOTE: requires Python 3.10+
"""

from __future__ import annotations

import argparse
import contextlib
import dataclasses
import datetime
import enum
import os
import sqlite3
import subprocess
import sys
import urllib.parse

import requests

from . import reddit, wayback

HN_SEARCH_URL = "https://hn.algolia.com/api/v1/search"


@enum.unique
class Command(enum.Enum):
    """User command."""

    ARCHIVE = "archive"
    DELETE = "delete"
    FAVORITE = "favorite"
    NEXT = "next"
    QUIT = "quit"

    @staticmethod
    def parse(text: str) -> Command | None:
        """Returns matching command, supports prefix matching, or None if not found."""
        if not text:
            # explicitly check empty string to avoid matching first command
            return None
        for cmd in Command:
            if cmd.value.startswith(text):
                return cmd
        return None


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


def format_discussions(data: dict, exclude_id: str | None = None) -> list[str]:
    """Returns formatted discussions."""
    items = [HNItem.from_json(item) for item in data["hits"]]
    items = [item for item in items if item.id != exclude_id]
    items = sorted(items, key=lambda item: item.points, reverse=True)
    return [str(item) for item in items]


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


def find_and_display_discussions(url: str, reddit_client: reddit.RedditClient) -> None:
    """https://hn.algolia.com/api."""
    parsed_url = urllib.parse.urlparse(url)
    if parsed_url.netloc == "news.ycombinator.com":
        post_ids = urllib.parse.parse_qs(parsed_url.query)["id"]
        assert len(post_ids) == 1, "expected HN story to have ID query parameter"
        post_id = post_ids[0]
        resp = requests.get(
            f"https://hacker-news.firebaseio.com/v0/item/{post_id}.json"
        )
        resp.raise_for_status()
        data = resp.json()
        if (item_url := data.get("url")) is not None:
            print(item_url)
            find_and_display_discussions_non_hn(item_url, exclude_id=post_id)
    elif parsed_url.netloc == "www.reddit.com":
        submission_id = parsed_url.path.split("/")[-3]
        submission = reddit_client.get_submission(submission_id)
        print(submission.url)
        find_and_display_discussions_non_hn(submission.url)
    else:
        find_and_display_discussions_non_hn(url)


def archive_item(mj_id: int) -> None:
    """Archives the item in Memory Jogger."""
    run_memory_jogger(["saved-items", "archive", "--item-id", str(mj_id)])


def favorite_item(mj_id: int) -> None:
    """Favorites the item in Memory Jogger."""
    run_memory_jogger(["saved-items", "favorite", "--item-id", str(mj_id)])


def delete_item(mj_id: int) -> None:
    """Deletes the item in Memory Jogger."""
    run_memory_jogger(["saved-items", "delete", "--item-id", str(mj_id)])


def run_memory_jogger(args: list[str]) -> None:
    """Runs the Memory Jogger command."""
    env = os.environ.copy()
    env["DATABASE_URL"] = os.environ["MEMORY_JOGGER_DATABASE_URL"]
    subprocess.run(["memory_jogger"] + args, check=True, env=env)


@dataclasses.dataclass
class MJSavedItem:
    """Saved item in Memory Jogger."""

    id: int
    title: str
    excerpt: str
    url: str
    time_added: str


def main() -> None:
    """CLI entrypoint."""
    parser = argparse.ArgumentParser()
    parser.parse_args()

    reddit_client = reddit.RedditClient()
    db_url = os.environ["MEMORY_JOGGER_DATABASE_URL"]
    # Memory Jogger requires sqlite:// prefix, but sqlite3.connect() does not support it
    db_url = db_url.removeprefix("sqlite://")
    with contextlib.closing(sqlite3.connect(db_url)) as con:
        cur = con.cursor()
        while True:
            cur.execute(
                "SELECT id,title,excerpt,url,time_added FROM saved_items ORDER BY RANDOM() LIMIT 1"
            )
            mj_item = MJSavedItem(*cur.fetchone())
            lines = [mj_item.title, ""]
            if mj_item.excerpt:
                lines.extend([mj_item.excerpt, ""])
            lines.append(mj_item.url)
            lines.append(f"added: {mj_item.time_added}")
            print("\n".join(lines))
            try:
                find_and_display_discussions(mj_item.url, reddit_client)
                time_added = datetime.datetime.fromisoformat(mj_item.time_added)
                if (url := wayback.get_snapshot(mj_item.url, time_added)) is not None:
                    print(f"{url} (wayback archive)")
            except requests.RequestException as exc:
                print(f"warning: fetching discussions failed: {exc}", file=sys.stderr)

            while True:
                try:
                    reply = input("(a)rchive (d)elete (f)avorite (n)ext (q)uit: ")
                except EOFError:
                    cmd: Command | None = Command.QUIT
                else:
                    if not reply:
                        # Re-prompt if empty
                        continue
                    cmd = Command.parse(reply)

                match cmd:
                    case Command.ARCHIVE:
                        archive_item(mj_item.id)
                        break
                    case Command.DELETE:
                        delete_item(mj_item.id)
                        break
                    case Command.FAVORITE:
                        favorite_item(mj_item.id)
                        # fall through to prompt for another action on this item
                    case Command.NEXT:
                        break
                    case Command.QUIT:
                        sys.exit()
                    case None:
                        print(f"unknown command: {reply}")
