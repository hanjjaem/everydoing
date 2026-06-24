# server.py
import json
import os
import subprocess
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
        self._serve_static(path)

    def _serve_static(self, path):
        if path == "/":
            path = "/index.html"
        full = os.path.normpath(os.path.join(STATIC_DIR, path.lstrip("/")))
        if not full.startswith(STATIC_DIR) or not os.path.isfile(full):
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
        if urlparse(self.path).path != "/api/open":
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", 0))
        data = json.loads(self.rfile.read(length).decode("utf-8"))
        target = data.get("path", "")
        if not os.path.exists(target):
            self._send_json(404, {"error": "파일을 찾을 수 없습니다"})
            return
        if data.get("mode") == "folder":
            subprocess.Popen(["explorer", "/select,", target])
        else:
            os.startfile(target)
        self._send_json(200, {"ok": True})


if __name__ == "__main__":
    port = CONFIG.get("server_port", 5000)
    print("http://localhost:{}".format(port))
    ThreadingHTTPServer(("localhost", port), Handler).serve_forever()
