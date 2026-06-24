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
