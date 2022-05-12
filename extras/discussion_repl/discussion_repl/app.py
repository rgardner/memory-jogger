"""Finds Hacker News discussions for a given article.

TODO:
- show nearest discussion first
"""

from __future__ import annotations

import argparse
import contextlib
import datetime
import os
import sqlite3
import sys
import urllib.parse

import requests

from discussion_repl import console, hn, memory_jogger, reddit, wayback


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
            hn.find_and_display_discussions_non_hn(item_url, exclude_id=post_id)
    elif parsed_url.netloc == "www.reddit.com":
        submission = reddit_client.get_submission(url)
        print(submission.url)
        hn.find_and_display_discussions_non_hn(submission.url)
    else:
        hn.find_and_display_discussions_non_hn(url)


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
            mj_item = memory_jogger.MJSavedItem(*cur.fetchone())
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
                    cmd = console.CommandPrompt.ask("enter a command")
                except (EOFError, KeyboardInterrupt):
                    cmd = console.Command.QUIT

                match cmd:
                    case console.Command.ARCHIVE:
                        memory_jogger.archive_item(mj_item.id)
                        break
                    case console.Command.DELETE:
                        memory_jogger.delete_item(mj_item.id)
                        break
                    case console.Command.FAVORITE:
                        memory_jogger.favorite_item(mj_item.id)
                        # fall through to prompt for another action on this item
                    case console.Command.NEXT:
                        break
                    case console.Command.QUIT:
                        sys.exit()
