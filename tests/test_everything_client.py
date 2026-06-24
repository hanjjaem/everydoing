from everything_client import build_search_query


def test_build_search_query_combines_ext_and_paths():
    q = build_search_query(
        scan_paths=["C:\\Users\\u\\Desktop", "C:\\Users\\u\\Docs"],
        extensions=["hwp", "docx"],
    )
    assert q == 'ext:hwp;docx ("C:\\Users\\u\\Desktop\\" | "C:\\Users\\u\\Docs\\")'


def test_build_search_query_single_path():
    q = build_search_query(scan_paths=["C:\\A"], extensions=["pdf"])
    assert q == 'ext:pdf ("C:\\A\\")'
