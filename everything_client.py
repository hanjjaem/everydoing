import json
import urllib.parse
import urllib.request
import urllib.error
from datetime import datetime, timedelta

_FILETIME_EPOCH = datetime(1601, 1, 1)


def build_search_query(scan_paths: list[str], extensions: list[str]) -> str:
    """Everything 검색 문자열 조립. 경로는 끝 백슬래시 유무와 무관하게 처리된다."""
    ext_part = "ext:" + ";".join(extensions)
    quoted = ['"' + p.rstrip("\\") + '\\"' for p in scan_paths]
    path_part = "(" + " | ".join(quoted) + ")"
    return ext_part + " " + path_part


def filetime_to_date(filetime: "int | str | None") -> "str | None":
    """Windows FILETIME (100ns ticks since 1601-01-01 UTC) to 'YYYY-MM-DD'.

    Args:
        filetime: int or str, FILETIME value (100-nanosecond ticks).

    Returns:
        str in 'YYYY-MM-DD' format, or None if empty/zero/unparseable.
    """
    try:
        ticks = int(filetime)
    except (TypeError, ValueError):
        return None
    if ticks <= 0:
        return None
    dt = _FILETIME_EPOCH + timedelta(microseconds=ticks / 10)
    return dt.strftime("%Y-%m-%d")


def group_by_date(results: "list[dict]") -> "dict[str, list[dict]]":
    """Group Everything search results by modified date.

    Args:
        results: list of dicts, each with keys 'name', 'path', 'date_modified'.

    Returns:
        dict mapping 'YYYY-MM-DD' (or 'unknown') to list of items with 'name' and 'path'.
    """
    grouped = {}
    for item in results:
        date = filetime_to_date(item.get("date_modified")) or "unknown"
        grouped.setdefault(date, []).append(
            {"name": item.get("name"), "path": item.get("path")}
        )
    return grouped


class EverythingConnectionError(Exception):
    pass


def load_config(path="config.json"):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def fetch_files(config):
    query = build_search_query(config["scan_paths"], config["extensions"])
    params = urllib.parse.urlencode({
        "search": query,
        "json": 1,
        "path_column": 1,
        "date_modified_column": 1,
    })
    url = "http://{host}:{port}/?{params}".format(
        host=config["everything_host"],
        port=config["everything_port"],
        params=params,
    )
    try:
        with urllib.request.urlopen(url, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, OSError) as e:
        raise EverythingConnectionError(str(e))
    return group_by_date(data.get("results", []))
