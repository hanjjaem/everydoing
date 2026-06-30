<p align="center">
  <img src="docs/poster.svg" alt="내가 한 일을, 날짜로 — 파일 수정일을 월 달력에 자동 정렬하는 Windows 데스크톱 앱" width="780">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows_10%2F11-0078D4?style=flat-square&logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Tauri-2-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Rust-CE422B?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/License-BSL_1.1-8A2BE2?style=flat-square" alt="BSL 1.1">
</p>

# 내가 한 일을 자동으로, 캘린더에서, 한 눈에.

#### ⚡ 한눈에

- PC 안의 업무 파일을 **파일 수정일 기준**으로 월 달력에 자동 정렬
- HWP · DOC · XLS · PPT · PDF를 **브랜드색 배지**로 즉시 구분
- **한 줄 메모** — 날짜 칸 클릭, 인라인 입력, 로컬 저장
- **월간 요약** — 활동일 · 문서 수 · 가장 바쁜 날 · 파일 유형 분포
- **AI 총평 (선택)** — 기본 꺼짐. 켜도 파일명만 전송, 매번 confirm

**스택** · Tauri 2 · Rust · 바닐라 HTML/CSS/JS · NTFS MFT (`ntfs-reader`)

---

## 어떻게 동작하나 (원리)

### 1) 핵심 아이디어

"언제 무슨 문서를 건드렸는지"는 파일 수정일에 이미 기록돼 있습니다. 별도 태그·분류 없이 **파일 수정일만 읽어** 달력에 배치합니다. NTFS MFT를 직접 순회하므로 Everything 같은 외부 프로그램이 전혀 필요 없습니다.

### 2) 인덱싱 흐름

```
PC 내 파일 (NTFS 드라이브)
    │
    │  ntfs-reader — MFT 직접 순회 (관리자 권한)
    │  scan_paths의 드라이브 자동 추출 → 해당 볼륨만 인덱싱
    │  비NTFS / 권한 실패 드라이브는 skipped 처리 후 계속
    ▼
index.json  (%APPDATA%\EveryDoing\)
    │
    │  Tauri 백엔드 → 프런트 emit
    ▼
달력 렌더링  (바닐라 JS)
    │  날짜 클릭 → 파일 목록 드로어
    │  빈 칸 클릭 → 한 줄 메모 인라인 편집
    ▼
memos.json  (%APPDATA%\EveryDoing\)
```

### 3) 데이터 저장

| 파일 | 위치 | 내용 |
|---|---|---|
| `config.json` | `%APPDATA%\EveryDoing\` | 스캔 경로·확장자·AI 옵션 |
| `index.json` | `%APPDATA%\EveryDoing\` | 파일 색인 캐시 |
| `memos.json` | `%APPDATA%\EveryDoing\` | 날짜별 한 줄 메모 |

### 4) AI 총평 — BYOK, 기본 꺼짐

```
설정에서 수동으로 켬
    │  Config.ai_summary_enabled = true
    │  사용자 본인 OpenAI 키 입력 → config.json에만 저장 (서버 미경유)
    ▼
사이드바 'AI 요약 생성' 버튼 클릭 + 매번 confirm 동의
    │  전송 대상: 파일명만 (파일 내용 아님)
    ▼
OpenAI API

끄면 → 외부 통신 0
```

---

## 코드 구조

```
src-tauri/src/
  main.rs          Tauri 커맨드 등록 · AppHandle 진입점
  indexer.rs       MFT 순회 · 파일 필터 · 날짜 변환
  store.rs         index.json 캐시 저장/로드
  summarizer.rs    AI 요약 (OpenAI 호출, 선택 기능)

static/
  index.html       달력 UI · 사이드바 · 드로어 · 메모 · 설정 패널
  app.js           이벤트 처리 · 달력 렌더 · Tauri invoke

src-tauri/
  tauri.conf.json  앱 설정 · 권한
  Cargo.toml       의존성 (ntfs-reader, tauri 2, tokio …)
```

---

## 로컬 실행

```bash
# 요구 사항
# - Windows 10 / 11 (x64)
# - Rust 1.77+  →  https://rustup.rs
# - Visual Studio Build Tools + WebView2  (Tauri 2 사전 요구사항)

# 개발 실행 — 반드시 관리자 터미널에서
cargo tauri dev

# 배포 빌드 — NSIS 설치본 (UAC 승격 포함)
cargo tauri build
```

> MFT 원시 접근은 관리자 권한이 필요합니다. 일반 터미널에서는 인덱싱이 동작하지 않습니다.

---

## 한계 · 주의

- 비NTFS 드라이브(USB FAT32 등)는 자동으로 건너뜁니다.
- 파일 수정일은 앱이 저장하는 날짜가 아니라 OS가 기록하는 날짜입니다. 복사·이동 시 바뀔 수 있습니다.
- AI 총평은 파일명만 보고 추론합니다. 최종 업무 판단은 사람이 합니다.

---

## 라이선스

[Business Source License 1.1](LICENSE) — © 2026 한쨈

비프로덕션(개발·학습·테스트) 자유 사용. **2030-06-29** 이후 Apache 2.0 자동 전환.
