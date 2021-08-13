#!/usr/bin/env python

"""Finds Hacker News discussions for a given article.

TODO:
- show nearest discussion first
- add Reddit support
- show Pocket URL and search fallback (see main.rs)

NOTE: requires Python 3.9+
"""

import argparse
import contextlib
import datetime
import enum
import os
import sqlite3
import subprocess
import urllib.parse
from typing import Optional

import requests

HN_SEARCH_URL = "https://hn.algolia.com/api/v1/search"


class Command(enum.Enum):
    ARCHIVE = "archive"
    DELETE = "delete"
    FAVORITE = "favorite"
    NEXT = "next"
    QUIT = "quit"

    @staticmethod
    def parse(s: str) -> Optional["Command"]:
        for cmd in Command:
            if cmd.value.startswith(s):
                return cmd


def find_url_submissions(url: str, exclude_id: Optional[str] = None) -> None:
    params = {
        "query": url,
        "restrictSearchableAttributes": "url",
    }
    r = requests.get(HN_SEARCH_URL, params=params)
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
        id = post["objectID"]
        discuss_url = f"https://news.ycombinator.com/item?id={id}"
        created_at = post["created_at_i"]
        created_date = datetime.date.fromtimestamp(created_at)
        points = f"{post['points']} point" + ("s" if post["points"] != 1 else "")
        print(f"{discuss_url} | {points} | {created_date.isoformat()}")


def display_discussions(url: str) -> None:
    """https://hn.algolia.com/api."""
    parsed_url = urllib.parse.urlparse(url)
    if parsed_url.netloc == "news.ycombinator.com":
        ids = urllib.parse.parse_qs(parsed_url.query)["id"]
        assert len(ids) == 1, "expected HN story to have ID query parameter"
        id = ids[0]
        r = requests.get(f"https://hacker-news.firebaseio.com/v0/item/{id}.json")
        data = r.json()
        if (item_url := data.get("url")) is not None:
            print(item_url)
            find_url_submissions(item_url, exclude_id=id)
    else:
        find_url_submissions(url)


def archive_item(id: int) -> None:
    subprocess.run(
        ["memory_jogger", "saved-items", "archive", "--item-id", str(id)], check=True
    )


def favorite_item(id: int) -> None:
    subprocess.run(
        ["memory_jogger", "saved-items", "favorite", "--item-id", str(id)], check=True
    )


def delete_item(id: int) -> None:
    subprocess.run(
        ["memory_jogger", "saved-items", "delete", "--item-id", str(id)], check=True
    )


def main():
    parser = argparse.ArgumentParser()
    parser.parse_args()

    db_url = os.environ["DATABASE_URL"]
    # Memory Jogger requires sqlite:// prefix, but sqlite3.connect() does not support it
    db_url = db_url.removeprefix("sqlite://")
    with contextlib.closing(sqlite3.connect(db_url)) as con:
        cur = con.cursor()
        while True:
            cur.execute("SELECT * FROM saved_items ORDER BY RANDOM() LIMIT 1")
            id, _, _, title, excerpt, url, _ = cur.fetchone()
            lines = [title, ""]
            if excerpt:
                lines.extend([excerpt, ""])
            lines.append(url)
            print("\n".join(lines))
            display_discussions(url)

            while True:
                reply = input("(a)rchive (d)elete (f)avorite (n)ext (q)uit: ")
                cmd = Command.parse(reply)
                if cmd == Command.FAVORITE:
                    favorite_item(id)
                elif cmd is None:
                    print(f"unknown command: {reply}")
                else:
                    break
            
            if cmd == Command.ARCHIVE:
                archive_item(id)
            elif cmd == Command.DELETE:
                delete_item(id)
            elif cmd == Command.NEXT:
                continue
            elif cmd == Command.QUIT:
                break


if __name__ == "__main__":
    main()
