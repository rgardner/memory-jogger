import discussion_repl


def test_wayback_parse_snapshot():
    url = "http://web.archive.org/web/20130919044612/http://example.com/"
    data = {
        "archived_snapshots": {
            "closest": {
                "available": True,
                "url": url,
                "timestamp": "20130919044612",
                "status": "200",
            }
        }
    }
    actual = discussion_repl.wayback.parse_url_from_snapshot(data)
    assert actual == url


def test_wayback_parse_snapshot_not_found():
    data = {"archived_snapshots": {}}
    actual = discussion_repl.wayback.parse_url_from_snapshot(data)
    assert actual is None
