def build_search_query(scan_paths: list[str], extensions: list[str]) -> str:
    """Everything 검색 문자열 조립. 경로는 끝 백슬래시 유무와 무관하게 처리된다."""
    ext_part = "ext:" + ";".join(extensions)
    quoted = ['"' + p.rstrip("\\") + '\\"' for p in scan_paths]
    path_part = "(" + " | ".join(quoted) + ")"
    return ext_part + " " + path_part
