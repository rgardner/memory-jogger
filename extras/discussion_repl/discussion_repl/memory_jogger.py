"""Memory Jogger commands."""

import dataclasses
import os
import subprocess


@dataclasses.dataclass
class MJSavedItem:
    """Saved item in Memory Jogger."""

    id: int
    title: str
    excerpt: str
    url: str
    time_added: str


def archive_item(mj_id: int) -> None:
    """Archives the item in Memory Jogger."""
    _run_memory_jogger(["saved-items", "archive", "--item-id", str(mj_id)])


def favorite_item(mj_id: int) -> None:
    """Favorites the item in Memory Jogger."""
    _run_memory_jogger(["saved-items", "favorite", "--item-id", str(mj_id)])


def delete_item(mj_id: int) -> None:
    """Deletes the item in Memory Jogger."""
    _run_memory_jogger(["saved-items", "delete", "--item-id", str(mj_id)])


def _run_memory_jogger(args: list[str]) -> None:
    """Runs the Memory Jogger command."""
    env = os.environ.copy()
    env["DATABASE_URL"] = os.environ["MEMORY_JOGGER_DATABASE_URL"]
    subprocess.run(["memory_jogger"] + args, check=True, env=env)
