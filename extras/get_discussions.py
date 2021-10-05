#!/usr/bin/env python

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
import datetime
import enum
import os
import sqlite3
import subprocess
import sys
import urllib.parse

import requests

HN_SEARCH_URL = "https://hn.algolia.com/api/v1/search"


@enum.unique
class Command(enum.Enum):
    ARCHIVE = "archive"
    DELETE = "delete"
    FAVORITE = "favorite"
    NEXT = "next"
    QUIT = "quit"

    @staticmethod
    def parse(text: str) -> Command | None:
        for cmd in Command:
            if cmd.value.startswith(text):
                return cmd
        return None


def find_url_submissions(url: str, exclude_id: str | None = None) -> None:
    """Finds HackerNews discussions for the given URL.

    :raises requests.RequestException: HN API request failed
    """
    params = {
        "query": url,
        "restrictSearchableAttributes": "url",
    }
    r = requests.get(HN_SEARCH_URL, params=params)
    r.raise_for_status()
    data = r.json()
    posts = sorted(
        (
            h
            for h in data["hits"]
            if h["num_comments"] != 0
            and (exclude_id is None or h["objectID"] != exclude_id)
        ),
        key=lambda h: h["points"],
        reverse=True,
    )
    for post in posts:
        post_id = post["objectID"]
        discuss_url = f"https://news.ycombinator.com/item?id={post_id}"
        created_at = post["created_at_i"]
        created_date = datetime.date.fromtimestamp(created_at)
        points = f"{post['points']} point" + ("s" if post["points"] != 1 else "")
        print(f"{discuss_url} | {points} | {created_date.isoformat()}")


def display_discussions(url: str) -> None:
    """https://hn.algolia.com/api."""
    parsed_url = urllib.parse.urlparse(url)
    if parsed_url.netloc == "news.ycombinator.com":
        post_ids = urllib.parse.parse_qs(parsed_url.query)["id"]
        assert len(post_ids) == 1, "expected HN story to have ID query parameter"
        post_id = post_ids[0]
        r = requests.get(f"https://hacker-news.firebaseio.com/v0/item/{post_id}.json")
        r.raise_for_status()
        data = r.json()
        if (item_url := data.get("url")) is not None:
            print(item_url)
            find_url_submissions(item_url, exclude_id=post_id)
    else:
        find_url_submissions(url)


def archive_item(mj_id: int) -> None:
    subprocess.run(
        ["memory_jogger", "saved-items", "archive", "--item-id", str(mj_id)], check=True
    )


def favorite_item(mj_id: int) -> None:
    subprocess.run(
        ["memory_jogger", "saved-items", "favorite", "--item-id", str(mj_id)],
        check=True,
    )


def delete_item(mj_id: int) -> None:
    subprocess.run(
        ["memory_jogger", "saved-items", "delete", "--item-id", str(mj_id)], check=True
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.parse_args()

    db_url = os.environ["DATABASE_URL"]
    # Memory Jogger requires sqlite:// prefix, but sqlite3.connect() does not support it
    db_url = db_url.removeprefix("sqlite://")
    with contextlib.closing(sqlite3.connect(db_url)) as con:
        cur = con.cursor()
        while True:
            cur.execute(
                "SELECT id,title,excerpt,url,time_added FROM saved_items ORDER BY RANDOM() LIMIT 1"
            )
            mj_id, title, excerpt, url, time_added = cur.fetchone()
            lines = [title, ""]
            if excerpt:
                lines.extend([excerpt, ""])
            lines.append(url)
            lines.append(f"added: {time_added}")
            print("\n".join(lines))
            try:
                display_discussions(url)
            except requests.RequestException as exc:
                print(f"error: fetching discussions failed: {exc}", file=sys.stderr)

            while True:
                reply = input("(a)rchive (d)elete (f)avorite (n)ext (q)uit: ")
                cmd = Command.parse(reply)
                match cmd:
                    case Command.FAVORITE:
                        favorite_item(mj_id)
                    case None:
                        print(f"unknown command: {reply}")
                    case _:
                        break

            match cmd:
                case Command.ARCHIVE:
                    archive_item(mj_id)
                case Command.DELETE:
                    delete_item(mj_id)
                case Command.NEXT:
                    continue
                case Command.QUIT:
                    break


if __name__ == "__main__":
    main()
