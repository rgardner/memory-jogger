"""Reddit API wrapper."""

import os

import praw
import praw.models

USER_AGENT = "discussion_repl by /u/rgardner"


# pylint: disable=too-few-public-methods
class RedditClient:
    """API wrapper for Reddit API."""

    def __init__(self) -> None:
        """Creates new RedditClient with app's credentials."""
        client_id = os.environ["MEMORY_JOGGER_REDDIT_CLIENT_ID"]
        client_secret = os.environ["MEMORY_JOGGER_REDDIT_CLIENT_SECRET"]
        self.reddit = praw.Reddit(
            client_id=client_id, client_secret=client_secret, user_agent=USER_AGENT
        )
        self.reddit.read_only = True

    def get_submission(self, submission_url: str) -> praw.models.Submission:
        """Gets a submission by submission URL."""
        return self.reddit.submission(url=submission_url)
