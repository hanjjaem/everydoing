import os
import tempfile

from everything_client import (
    build_search_query,
    filetime_to_date,
    group_by_date,
    add_scan_path,
    remove_scan_path,
    save_config,
    load_config,
)


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
    # 2026-06-12 00:00:00 UTC == 2026-06-12 09:00:00 KST → local date 2026-06-12
    assert filetime_to_date(134256960000000000) == "2026-06-12"


def test_filetime_to_date_uses_local_timezone():
    # FILETIME for 2026-06-11 20:00 UTC == 2026-06-12 05:00 KST → must group as 2026-06-12
    assert filetime_to_date(134256816000000000) == "2026-06-12"


def test_filetime_to_date_accepts_string():
    assert filetime_to_date("134256960000000000") == "2026-06-12"


def test_filetime_to_date_empty_returns_none():
    assert filetime_to_date("") is None
    assert filetime_to_date(0) is None
    assert filetime_to_date("notanumber") is None


def test_group_by_date_groups_and_keeps_name_path():
    results = [
        {"name": "a.hwp", "path": "C:\\x\\a.hwp", "date_modified": 134256960000000000},
        {"name": "b.docx", "path": "C:\\x\\b.docx", "date_modified": 134256960000000000},
    ]
    grouped = group_by_date(results)
    assert grouped == {
        "2026-06-12": [
            {"name": "a.hwp", "path": "C:\\x\\a.hwp"},
            {"name": "b.docx", "path": "C:\\x\\b.docx"},
        ]
    }


def test_group_by_date_unknown_bucket():
    results = [{"name": "c.pdf", "path": "C:\\c.pdf", "date_modified": ""}]
    grouped = group_by_date(results)
    assert grouped == {"unknown": [{"name": "c.pdf", "path": "C:\\c.pdf"}]}


def test_add_scan_path_appends_new():
    assert add_scan_path(["C:\\A"], "C:\\B") == ["C:\\A", "C:\\B"]


def test_add_scan_path_ignores_duplicate():
    assert add_scan_path(["C:\\A"], "C:\\A") == ["C:\\A"]


def test_add_scan_path_ignores_empty():
    assert add_scan_path(["C:\\A"], "") == ["C:\\A"]


def test_remove_scan_path_removes():
    assert remove_scan_path(["C:\\A", "C:\\B"], "C:\\A") == ["C:\\B"]


def test_remove_scan_path_absent_noop():
    assert remove_scan_path(["C:\\A"], "C:\\X") == ["C:\\A"]


def test_save_then_load_config_roundtrip():
    cfg = {"scan_paths": ["C:\\A"], "server_port": 5000}
    fd, path = tempfile.mkstemp(suffix=".json")
    os.close(fd)
    try:
        save_config(cfg, path)
        assert load_config(path) == cfg
    finally:
        os.remove(path)
