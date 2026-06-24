from everything_client import build_search_query, filetime_to_date


def test_build_search_query_combines_ext_and_paths():
    q = build_search_query(
        scan_paths=["C:\\Users\\u\\Desktop", "C:\\Users\\u\\Docs"],
        extensions=["hwp", "docx"],
    )
    assert q == 'ext:hwp;docx ("C:\\Users\\u\\Desktop\\" | "C:\\Users\\u\\Docs\\")'


def test_build_search_query_single_path():
    q = build_search_query(scan_paths=["C:\\A"], extensions=["pdf"])
    assert q == 'ext:pdf ("C:\\A\\")'


def test_filetime_to_date_known_value():
    # 2026-06-12 00:00:00 UTC == FILETIME 134256960000000000
    assert filetime_to_date(134256960000000000) == "2026-06-12"


def test_filetime_to_date_accepts_string():
    assert filetime_to_date("134256960000000000") == "2026-06-12"


def test_filetime_to_date_empty_returns_none():
    assert filetime_to_date("") is None
    assert filetime_to_date(0) is None
    assert filetime_to_date("notanumber") is None
