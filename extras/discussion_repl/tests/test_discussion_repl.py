import datetime

import pytest

import discussion_repl


def test_version():
    assert discussion_repl.__version__ == "0.1.0"


def test_command_parse_unique_prefix():
    commands = list(discussion_repl.Command)
    prefixes = {cmd.value[0] for cmd in commands}
    assert len(prefixes) == len(commands)


CMD_PREFIXES = [
    (cmd, cmd.value[: i + 1])
    for cmd in discussion_repl.Command
    for i in range(len(cmd.value))
]


@pytest.mark.parametrize("expected_cmd,prefix", CMD_PREFIXES)
def test_command_parse_supports_prefixes(expected_cmd, prefix):
    actual_cmd = discussion_repl.Command.parse(prefix)
    assert actual_cmd == expected_cmd


@pytest.mark.parametrize("text", ["", "unknown"])
def test_command_parse_unknown_returns_none(text):
    actual = discussion_repl.Command.parse(text)
    assert actual is None


def test_hn_item_from_json():
    data = {
        "objectID": "1",
        "points": 10,
        "created_at_i": 0,
    }
    actual = discussion_repl.HNItem.from_json(data)
    expected_created_at = datetime.date(1969, 12, 31)
    expected = discussion_repl.HNItem(id="1", points=10, created_at=expected_created_at)
    assert actual == expected


def test_hn_item_discussion_url():
    expected_created_at = datetime.date(1970, 1, 1)
    item = discussion_repl.HNItem(id="1", points=10, created_at=expected_created_at)
    assert item.discussion_url == "https://news.ycombinator.com/item?id=1"
