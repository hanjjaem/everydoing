import json
import urllib.parse
import urllib.request
import urllib.error
from datetime import datetime

_FILETIME_POSIX_OFFSET = 11644473600  # seconds between 1601-01-01 and 1970-01-01


def build_search_query(scan_paths: list[str], extensions: list[str]) -> str:
    """Everything 검색 문자열 조립. 경로는 끝 백슬래시 유무와 무관하게 처리된다."""
    ext_part = "ext:" + ";".join(extensions)
    quoted = ['"' + p.rstrip("\\") + '\\"' for p in scan_paths]
    path_part = "(" + " | ".join(quoted) + ")"
    return ext_part + " " + path_part


def filetime_to_date(filetime: "int | str | None") -> "str | None":
    """Windows FILETIME(1601 기준 100ns)을 로컬 시간 기준 'YYYY-MM-DD'로 변환. 불가 시 None."""
    try:
        ticks = int(filetime)
    except (TypeError, ValueError):
        return None
    if ticks <= 0:
        return None
    posix = ticks / 10**7 - _FILETIME_POSIX_OFFSET
    try:
        return datetime.fromtimestamp(posix).strftime("%Y-%m-%d")
    except (OSError, ValueError, OverflowError):
        return None


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


def load_config(path: str = "config.json") -> dict:
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def fetch_files(config: dict) -> "dict[str, list[dict]]":
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
    except (urllib.error.URLError, OSError, json.JSONDecodeError) as e:
        raise EverythingConnectionError(str(e))
    return group_by_date(data.get("results", []))
