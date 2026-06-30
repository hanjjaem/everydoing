<h1 align="center">EveryDoing</h1>

<p align="center">
  <b>내가 한 일을, 수정한 날짜 기준으로 월 달력에 자동 정리</b><br/>
  "언제 무엇을 했는지"를 한눈에 보는 개인용 Windows 데스크톱 앱
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows-0078D4" alt="Windows" />
  <img src="https://img.shields.io/badge/Tauri-2-24C8D8" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/License-BSL%201.1-blue" alt="BSL 1.1" />
</p>

---

## 이런 걸 합니다

- **수정일 기준 달력** — 스캔 폴더 안의 업무 문서(HWP·DOC·XLS·PPT·PDF)를 파일 수정일로 묶어 월 달력에 표시합니다.
- **파일 종류 배지** — 한글·워드·엑셀·PPT·PDF를 브랜드색 배지로 구분.
- **한 줄 메모** — 날짜 칸에 그날의 메모를 인라인으로 적습니다. (로컬 저장)
- **월간요약 모달** — 그 달의 활동일·문서 수·가장 바쁜 날·파일 유형 분포를 한 화면에. 키워드는 파일명에서 로컬로 추출.
- **AI 월간 총평 (선택)** — 켜면 그 달 파일명을 바탕으로 한 줄 총평을 생성. **기본 꺼짐**.

## 인덱싱 방식

NTFS MFT를 직접 순회(`ntfs-reader`)해 색인하므로 Everything 같은 외부 프로그램이 필요 없습니다. 시작 시 1회 스캔하고, "새로고침"으로 다시 스캔합니다.

## 설치 / 빌드

Windows 전용입니다. MFT 원시 접근을 위해 **관리자 권한**이 필요합니다.

**요구 사항**
- Windows 10/11 (x64)
- [Rust](https://rustup.rs) + [Tauri 2 사전 요구사항](https://tauri.app/start/prerequisites/) (Visual Studio Build Tools, WebView2)

**개발 실행** (관리자 터미널에서)
```bash
cargo tauri dev
```

**배포 빌드** (NSIS 설치본, UAC 승격 포함)
```bash
cargo tauri build
```

설정(스캔 폴더·확장자·AI 옵션)은 `%APPDATA%\EveryDoing\config.json`에 저장됩니다.

## 보안 & 데이터 흐름

이 앱은 **기본적으로 외부와 통신하지 않습니다.** 색인·달력·메모·월간 통계·키워드는 전부 로컬에서 처리됩니다.

| 기능 | 데이터 위치 | 외부 전송 |
|------|------------|----------|
| 파일 색인 (MFT) | 로컬 | 없음 |
| 메모 | `%APPDATA%\EveryDoing\memos.json` | 없음 |
| 월간 통계·키워드 추출 | 로컬 | 없음 |
| **AI 월간 총평** (선택) | OpenAI API | **파일명만** 전송 (켰을 때만) |

- **AI 요약은 기본 꺼짐(opt-in).** 설정에서 켜야 동작하며, 끄면 외부 통신이 전혀 없습니다.
- **BYOK** — 사용자가 자기 OpenAI API 키를 입력하며, 키는 이 PC의 `config.json`에만 저장됩니다(서버 미경유).
- **on-demand** — 로드·스캔 시 자동 전송하지 않습니다. 버튼을 누르고 매번 동의(confirm)했을 때만 전송하며, 전송 대상은 **파일명뿐**(파일 내용 아님)입니다.

> 공공기관 등 폐쇄망·보안 환경에서는 AI 요약을 끈 채로 순수 로컬 도구로 사용하세요.

## 라이선스

[Business Source License 1.1](LICENSE) — Copyright (c) 2026 한쨈

- 비프로덕션(개발·테스트·학습) 자유 사용
- 프로덕션/상용은 별도 라이선스 필요
- **2030-06-29** 이후 Apache License 2.0으로 자동 전환
