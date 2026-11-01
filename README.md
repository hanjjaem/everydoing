<h1 align="center">EveryDoing</h1>

<p align="center">내가 한 일을, 날짜로</p>

<p align="center">
  <a href="docs/everydoing-story.mp4"><img src="docs/everydoing-story.gif" alt="EveryDoing 소개 영상: 작업한 파일이 날짜별 기록이 됩니다" width="800"></a>
</p>

<p align="center">기억나지 않는 하루도, 다시 만날 수 있도록.</p>

<p align="center"><a href="docs/index.html">소개 페이지 소스 보기</a></p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows_10%2F11-0078D4?style=flat-square&logo=windows&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/badge/Tauri-2-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Rust-CE422B?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/License-BSL_1.1-8A2BE2?style=flat-square" alt="BSL 1.1">
</p>

<br>

<p align="center">
  업무 파일의 수정일을 자동으로 읽어 월 달력에 펼쳐 보여줍니다.<br>
  별도 태그·분류 없이, <strong>"그날 뭘 했더라"</strong>에 즉시 답합니다.
</p>

<br>

---

<table width="100%">
<tr>
<td width="33%" align="center" valign="top">

**📅 수정일 기준 달력**

파일 수정일을 자동으로 읽어\
월 달력에 배치합니다.\
어느 날 어떤 문서를 건드렸는지\
한눈에 확인합니다.

</td>
<td width="33%" align="center" valign="top">

**⚡ NTFS MFT 직접 인덱싱**

Everything 같은 외부 앱 없이\
NTFS MFT를 직접 순회합니다.\
수만 개 파일도 빠르게,\
단독 `.exe` 하나로.

</td>
<td width="33%" align="center" valign="top">

**🔒 완전 로컬**

색인·달력·메모·통계 모두\
이 PC에서만 처리됩니다.\
기본 상태에서\
외부 통신이 전혀 없습니다.

</td>
</tr>
<tr>
<td width="33%" align="center" valign="top">

**🏷 파일 종류 배지**

HWP · DOC · XLS · PPT · PDF를\
브랜드 컬러 배지로 구분합니다.\
어떤 종류의 작업을 했는지\
즉시 파악합니다.

</td>
<td width="33%" align="center" valign="top">

**✏️ 한 줄 메모**

날짜 칸 빈 곳을 클릭하면\
바로 메모를 입력합니다.\
로컬에 저장되고\
언제든 수정합니다.

</td>
<td width="33%" align="center" valign="top">

**✨ AI 총평 (선택)**

기본 꺼짐. 켜도\
파일명만 전송,\
매번 confirm 동의,\
본인 OpenAI 키(BYOK).

</td>
</tr>
</table>

---

## 어떻게 동작하나

```
PC 내 파일 (NTFS 드라이브)
    │  ntfs-reader — MFT 직접 순회
    │  scan_paths의 드라이브에서 해당 볼륨만 인덱싱
    ▼
index.json  (%APPDATA%\EveryDoing\)
    │  Tauri 백엔드 → 프런트 emit
    ▼
달력 렌더링  (바닐라 JS)
    │  날짜 클릭 → 파일 목록 드로어
    │  빈 칸 클릭 → 한 줄 메모 인라인 편집
    ▼
memos.json  (%APPDATA%\EveryDoing\)
```

> MFT 원시 접근은 관리자 권한이 필요합니다. 비NTFS 드라이브(USB FAT32 등)는 자동으로 건너뜁니다.

---

## 빠른 시작

```bash
# 요구사항: Windows 10/11, Rust 1.77+, VS Build Tools + WebView2

# 개발 실행 — 반드시 관리자 터미널에서
cargo tauri dev

# 배포 빌드 (NSIS 설치본, UAC 승격 포함)
cargo tauri build
```

설정 파일 위치: `%APPDATA%\EveryDoing\config.json`

설치본 생성부터 검증과 수동 배포까지의 자세한 절차는 [Windows 배포 가이드](docs/배포-가이드.md)를 참고하세요.

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
```

---

## 라이선스

[Business Source License 1.1](LICENSE) — © 2026 한쨈

비프로덕션(개발·학습·테스트) 자유 사용. **2030-06-29** 이후 Apache 2.0 자동 전환.
