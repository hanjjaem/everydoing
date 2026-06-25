# server.py
import json
import os
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse

import everything_client as ec

CONFIG = ec.load_config()
STATIC_DIR = os.path.join(os.path.dirname(__file__), "static")


class Handler(BaseHTTPRequestHandler):
    def _send_json(self, code, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = urlparse(self.path).path
        if path == "/api/files":
            try:
                self._send_json(200, ec.fetch_files(CONFIG))
            except ec.EverythingConnectionError as e:
                self._send_json(503, {"error": str(e)})
            return
        if path == "/api/config":
            self._send_json(200, {
                "scan_paths": CONFIG.get("scan_paths", []),
                "extensions": CONFIG.get("extensions", []),
            })
            return
        if path == "/api/memos":
            self._send_json(200, ec.load_memos())
            return
        self._serve_static(path)

    def _serve_static(self, path):
        if path == "/":
            path = "/index.html"
        full = os.path.normpath(os.path.join(STATIC_DIR, path.lstrip("/")))
        if (full != STATIC_DIR and not full.startswith(STATIC_DIR + os.sep)) or not os.path.isfile(full):
            self.send_error(404)
            return
        ctype = "text/html" if full.endswith(".html") else \
            "application/javascript" if full.endswith(".js") else "text/plain"
        with open(full, "rb") as f:
            body = f.read()
        self.send_response(200)
        self.send_header("Content-Type", ctype + "; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        route = urlparse(self.path).path
        if route not in ("/api/open", "/api/config", "/api/pick-folder", "/api/memos"):
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length) if length else b""
        try:
            data = json.loads(raw.decode("utf-8")) if raw else {}
        except (json.JSONDecodeError, UnicodeDecodeError):
            self._send_json(400, {"error": "invalid body"})
            return
        if route == "/api/open":
            self._handle_open(data)
        elif route == "/api/config":
            self._handle_save_config(data)
        elif route == "/api/memos":
            self._handle_save_memo(data)
        else:
            self._handle_pick_folder()

    def _handle_open(self, data):
        target = data.get("path", "")
        if not os.path.exists(target):
            self._send_json(404, {"error": "파일을 찾을 수 없습니다"})
            return
        if data.get("mode") == "folder":
            subprocess.Popen(["explorer", "/select,", target])
        else:
            os.startfile(target)
        self._send_json(200, {"ok": True})

    def _handle_save_config(self, data):
        global CONFIG
        scan_paths = data.get("scan_paths")
        if not isinstance(scan_paths, list):
            self._send_json(400, {"error": "scan_paths(list) 필요"})
            return
        if not scan_paths:
            self._send_json(400, {"error": "스캔 폴더를 최소 1개 지정하세요"})
            return
        extensions = data.get("extensions")
        if extensions is not None and not isinstance(extensions, list):
            self._send_json(400, {"error": "extensions(list) 필요"})
            return
        norm_exts = None
        if extensions is not None:
            norm_exts = ec.normalize_extensions(extensions)
            if not norm_exts:
                self._send_json(400, {"error": "파일 유형을 최소 1개 선택하세요"})
                return
        try:
            cfg = ec.load_config()
            cfg["scan_paths"] = scan_paths
            if norm_exts is not None:
                cfg["extensions"] = norm_exts
            ec.save_config(cfg)
            CONFIG = cfg
        except OSError as e:
            self._send_json(500, {"error": str(e)})
            return
        self._send_json(200, {"ok": True})

    def _handle_pick_folder(self):
        helper = os.path.join(os.path.dirname(__file__), "pick_folder.py")
        try:
            out = subprocess.run(
                [sys.executable, helper],
                capture_output=True, text=True, timeout=120,
            )
        except (OSError, subprocess.SubprocessError) as e:
            self._send_json(500, {"error": str(e)})
            return
        if out.returncode != 0:
            self._send_json(500, {"error": out.stderr.strip() or "folder picker failed"})
            return
        self._send_json(200, {"path": out.stdout.strip()})

    def _handle_save_memo(self, data):
        date = data.get("date")
        if not isinstance(date, str) or not date:
            self._send_json(400, {"error": "date 필요"})
            return
        text = data.get("text", "")
        try:
            memos = ec.set_memo(ec.load_memos(), date, text if isinstance(text, str) else "")
            ec.save_memos(memos)
        except OSError as e:
            self._send_json(500, {"error": str(e)})
            return
        self._send_json(200, {"ok": True})


if __name__ == "__main__":
    port = CONFIG.get("server_port", 5000)
    print("http://localhost:{}".format(port))
    ThreadingHTTPServer(("localhost", port), Handler).serve_forever()
