//! Local-only browser development bridge. This is not the shipping Tauri application.
#[path = "../indexer.rs"]
mod indexer;
#[path = "../store.rs"]
mod store;

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

struct State {
    dir: PathBuf,
    config: store::Config,
    scan: Option<indexer::ScanResult>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct WebCache {
    scan_paths: Vec<String>,
    extensions: Vec<String>,
    result: indexer::ScanResult,
}

fn scan_config(cfg: &store::Config) -> indexer::ScanResult {
    let roots: Vec<_> = cfg.scan_paths.iter().filter(|p| {
        let trimmed = p.trim_end_matches('\\');
        trimmed.len() == 2 && trimmed.as_bytes()[1] == b':'
    }).cloned().collect();
    let folders: Vec<_> = cfg.scan_paths.iter()
        .filter(|p| !roots.contains(p) && !indexer::path_matches(p, &roots))
        .cloned().collect();
    let mut combined = indexer::ScanResult { files: BTreeMap::new(), skipped: Vec::new() };
    if !roots.is_empty() {
        let mut root_cfg = cfg.clone();
        root_cfg.scan_paths = roots;
        let result = indexer::scan_without_progress(&root_cfg);
        combined.files = result.files;
        combined.skipped = result.skipped;
    }
    if !folders.is_empty() {
        let mut folder_cfg = cfg.clone();
        folder_cfg.scan_paths = folders;
        let result = indexer::scan_walk(&folder_cfg);
        for (date, files) in result.files { combined.files.entry(date).or_default().extend(files); }
        combined.skipped.extend(result.skipped);
    }
    combined
}

fn read_map(path: &Path) -> Result<BTreeMap<String, String>, String> {
    match std::fs::read(path) {
        Ok(data) => serde_json::from_slice(&data).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

fn invoke(state: &mut State, cmd: &str, args: &Value) -> Result<Value, String> {
    let get = |name: &str| args.get(name).and_then(Value::as_str).unwrap_or("");
    let data_path = |name: &str| state.dir.join(name);
    match cmd {
        "get_config" => serde_json::to_value(&state.config).map_err(|e| e.to_string()),
        "save_config" => {
            if args.get("aiSummaryEnabled").and_then(Value::as_bool).unwrap_or(false)
                || !get("openaiApiKey").is_empty() {
                return Err("브라우저 개발 모드에서는 AI 요약을 지원하지 않습니다. 데스크톱 앱에서 이용해 주세요.".into());
            }
            let paths: Vec<String> = serde_json::from_value(args.get("scanPaths").cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())?;
            let exts: Vec<String> = serde_json::from_value(args.get("extensions").cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())?;
            let (scan_paths, extensions) = store::validate_config(paths, exts)?;
            let config = store::Config { scan_paths, extensions, ..Default::default() };
            write_json(&data_path("config.json"), &config)?;
            state.config = config;
            state.scan = None;
            Ok(Value::Null)
        }
        "load_cache" => Ok(state.scan.as_ref().map(|s| json!(s)).unwrap_or(Value::Null)),
        "start_scan" => {
            let result = scan_config(&state.config);
            if !result.skipped.is_empty() {
                return Err(format!("스캔을 완료하지 못했습니다: {}", result.skipped.iter().take(3).cloned().collect::<Vec<_>>().join(", ")));
            }
            let cache = WebCache { scan_paths: state.config.scan_paths.clone(), extensions: state.config.extensions.clone(), result: result.clone() };
            write_json(&data_path("index.json"), &cache)?;
            state.scan = Some(result.clone());
            Ok(json!(result))
        }
        "get_memos" => Ok(json!(read_map(&data_path("memos.json"))?)),
        "save_memo" => {
            let date = get("date");
            if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() { return Err("올바른 날짜가 아닙니다.".into()); }
            let path = data_path("memos.json");
            let next = store::set_memo(&read_map(&path)?, date, get("text"));
            write_json(&path, &next)?;
            Ok(Value::Null)
        }
        "get_summaries" | "get_monthly_summaries" => Ok(json!({})),
        "save_summary" | "generate_summaries" | "generate_monthly_summary" => Err("브라우저 개발 모드에서는 AI 요약을 지원하지 않습니다.".into()),
        "prune_deleted" => {
            if let Some(ref mut scan) = state.scan {
                for files in scan.files.values_mut() { files.retain(|f| Path::new(&f.path).exists()); }
            }
            Ok(state.scan.as_ref().map(|s| json!(s)).unwrap_or(Value::Null))
        }
        "open_path" => {
            let path = get("path");
            let allowed = state.scan.as_ref().map(|s| s.files.values().flatten().any(|f| f.path == path)).unwrap_or(false);
            if !allowed || !Path::new(path).exists() { return Err("스캔 결과에 있는 파일만 열 수 있습니다.".into()); }
            if get("mode") == "folder" {
                std::process::Command::new("explorer").arg("/select,").arg(path).spawn().map_err(|e| e.to_string())?;
            } else {
                opener::open(path).map_err(|e| e.to_string())?;
            }
            Ok(Value::Null)
        }
        "copy_text" => {
            let text = get("text");
            let allowed = state.scan.as_ref().map(|s| s.files.values().flatten().any(|f| f.path == text)).unwrap_or(false);
            if !allowed { return Err("스캔 결과에 있는 경로만 복사할 수 있습니다.".into()); }
            let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
            cb.set_text(text).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        _ => Err(format!("지원하지 않는 명령: {cmd}")),
    }
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n", body.len())?;
    stream.write_all(body)
}

fn handle(mut stream: TcpStream, state: &mut State, port: u16, token: &str, root: &Path) -> Result<(), String> {
    stream.set_read_timeout(Some(Duration::from_secs(15))).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut first = String::new();
    reader.read_line(&mut first).map_err(|e| e.to_string())?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let mut headers = BTreeMap::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 { return Err("요청이 끝났습니다.".into()); }
        if line == "\r\n" { break; }
        if let Some((key, value)) = line.split_once(':') { headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string()); }
        if headers.len() > 64 { return Err("요청 헤더가 너무 많습니다.".into()); }
    }
    let host = format!("127.0.0.1:{port}");
    if headers.get("host") != Some(&host) { respond(&mut stream, "403 Forbidden", "text/plain", b"Forbidden").map_err(|e| e.to_string())?; return Ok(()); }
    if method == "GET" {
        let (data, ty) = match path {
            "/" | "/index.html" => {
                let html = std::fs::read_to_string(root.join("static/index.html")).map_err(|e| e.to_string())?;
                let html = html.replace("<script src=\"app.js\"></script>", &format!("<meta name=\"everydoing-session\" content=\"{token}\"><script src=\"/bridge.js\"></script><script src=\"/app.js\"></script>"));
                (html.into_bytes(), "text/html; charset=utf-8")
            }
            "/app.js" => (std::fs::read(root.join("static/app.js")).map_err(|e| e.to_string())?, "text/javascript; charset=utf-8"),
            "/bridge.js" => (std::fs::read(root.join("dev-web/bridge.js")).map_err(|e| e.to_string())?, "text/javascript; charset=utf-8"),
            _ => { respond(&mut stream, "404 Not Found", "text/plain", b"Not found").map_err(|e| e.to_string())?; return Ok(()); }
        };
        respond(&mut stream, "200 OK", ty, &data).map_err(|e| e.to_string())?;
    } else if method == "POST" && path == "/api/invoke" {
        let origin = format!("http://{host}");
        if headers.get("origin") != Some(&origin) || headers.get("x-everydoing-session").map(String::as_str) != Some(token)
            || headers.get("content-type").map(String::as_str) != Some("application/json") {
            respond(&mut stream, "403 Forbidden", "text/plain", b"Forbidden").map_err(|e| e.to_string())?; return Ok(());
        }
        let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        if len == 0 || len > 1_000_000 { return Err("요청 크기가 잘못되었습니다.".into()); }
        let mut body = vec![0; len]; reader.read_exact(&mut body).map_err(|e| e.to_string())?;
        let input: Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
        let cmd = input.get("command").and_then(Value::as_str).unwrap_or("");
        let args = input.get("args").cloned().unwrap_or(json!({}));
        let (status, output) = match invoke(state, cmd, &args) { Ok(result) => ("200 OK", json!({"result": result})), Err(error) => ("400 Bad Request", json!({"error": error})) };
        respond(&mut stream, status, "application/json; charset=utf-8", output.to_string().as_bytes()).map_err(|e| e.to_string())?;
    } else { respond(&mut stream, "404 Not Found", "text/plain", b"Not found").map_err(|e| e.to_string())?; }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().ok_or("프로젝트 경로 오류")?.to_path_buf();
    let dir = root.join("src-tauri/target/everydoing-web");
    std::fs::create_dir_all(&dir)?;
    let config_path = dir.join("config.json");
    let mut config: store::Config = match std::fs::read(&config_path) {
        Ok(data) => serde_json::from_slice(&data)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let existing = store::load_config().unwrap_or_default();
            store::Config { scan_paths: existing.scan_paths, extensions: existing.extensions, ..Default::default() }
        }
        Err(e) => return Err(e.into()),
    };
    config.ai_summary_enabled = false; config.openai_api_key.clear();
    let cached = std::fs::read(dir.join("index.json")).ok()
        .and_then(|data| serde_json::from_slice::<WebCache>(&data).ok())
        .filter(|cache| cache.scan_paths == config.scan_paths && cache.extensions == config.extensions)
        .map(|cache| cache.result)
        .or_else(|| store::load_index(&config));
    let mut state = State { dir, config, scan: cached };
    let requested_port = std::env::args().skip(1).find_map(|arg| arg.strip_prefix("--port=").map(str::to_owned))
        .map(|s| s.parse::<u16>()).transpose()?.unwrap_or(0);
    let listener = TcpListener::bind(("127.0.0.1", requested_port))?;
    let port = listener.local_addr()?.port();
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| std::io::Error::other(format!("세션 생성 실패: {e}")))?;
    let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    println!("EveryDoing 브라우저 개발 모드: http://127.0.0.1:{port}/");
    println!("메모와 설정은 데스크톱 앱과 별도 저장됩니다. 종료하려면 Ctrl+C.");
    for stream in listener.incoming() {
        match stream { Ok(stream) => { if let Err(e) = handle(stream, &mut state, port, &token, &root) { eprintln!("요청 오류: {e}"); } }, Err(e) => eprintln!("연결 오류: {e}") }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_folder_scan_and_memo_round_trip() {
        let dir = std::env::temp_dir().join(format!("everydoing-web-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("work.pdf");
        std::fs::write(&file, b"test document").unwrap();
        let config = store::Config {
            scan_paths: vec![dir.to_string_lossy().to_string()],
            extensions: vec!["pdf".into()],
            ..Default::default()
        };
        let mut state = State { dir: dir.clone(), config, scan: None };
        let scan = invoke(&mut state, "start_scan", &json!({})).unwrap();
        let files = scan["files"].as_object().unwrap();
        assert!(files.values().flat_map(|v| v.as_array().unwrap()).any(|v| v["name"] == "work.pdf"));
        let saved: WebCache = serde_json::from_slice(&std::fs::read(dir.join("index.json")).unwrap()).unwrap();
        assert_eq!(saved.result.files.values().map(Vec::len).sum::<usize>(), 1);
        invoke(&mut state, "save_memo", &json!({"date":"2026-11-10", "text":"verify"})).unwrap();
        assert_eq!(invoke(&mut state, "get_memos", &json!({})).unwrap()["2026-11-10"], "verify");
        assert!(invoke(&mut state, "open_path", &json!({"path":"C:\\unscanned.txt"})).is_err());
        std::fs::remove_file(file).unwrap();
        std::fs::remove_file(dir.join("memos.json")).unwrap();
        std::fs::remove_file(dir.join("index.json")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
