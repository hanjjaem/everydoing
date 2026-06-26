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
    normalize_extension,
    normalize_extensions,
    set_memo,
    load_memos,
    save_memos,
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


def test_normalize_extension_strips_dot_and_lowercases():
    assert normalize_extension(" .HWP ") == "hwp"


def test_normalize_extension_plain():
    assert normalize_extension("docx") == "docx"


def test_normalize_extension_empty_returns_none():
    assert normalize_extension("") is None
    assert normalize_extension("   ") is None
    assert normalize_extension(".") is None


def test_normalize_extensions_dedupes_keeps_order_drops_none():
    assert normalize_extensions(["PDF", ".pdf", "hwp", "", "docx"]) == ["pdf", "hwp", "docx"]


def test_set_memo_adds_new():
    assert set_memo({}, "2026-06-25", "예산안 검토") == {"2026-06-25": "예산안 검토"}


def test_set_memo_overwrites():
    out = set_memo({"2026-06-25": "old"}, "2026-06-25", "new")
    assert out == {"2026-06-25": "new"}


def test_set_memo_strips_whitespace():
    assert set_memo({}, "2026-06-25", "  보고서  ") == {"2026-06-25": "보고서"}


def test_set_memo_blank_deletes_key():
    assert set_memo({"2026-06-25": "x"}, "2026-06-25", "   ") == {}


def test_set_memo_blank_missing_key_noop():
    assert set_memo({"2026-06-24": "y"}, "2026-06-25", "") == {"2026-06-24": "y"}


def test_set_memo_does_not_mutate_original():
    original = {"2026-06-25": "x"}
    set_memo(original, "2026-06-25", "z")
    assert original == {"2026-06-25": "x"}


def test_load_memos_missing_file_returns_empty():
    assert load_memos("does_not_exist_memos.json") == {}


def test_save_then_load_memos_roundtrip():
    memos = {"2026-06-25": "한글 메모", "2026-06-24": "두 번째"}
    fd, path = tempfile.mkstemp(suffix=".json")
    os.close(fd)
    try:
        save_memos(memos, path)
        assert load_memos(path) == memos
    finally:
        os.remove(path)


def test_load_memos_corrupt_file_returns_empty():
    fd, path = tempfile.mkstemp(suffix=".json")
    os.close(fd)
    try:
        with open(path, "w", encoding="utf-8") as f:
            f.write("{not valid json")
        assert load_memos(path) == {}
    finally:
        os.remove(path)
