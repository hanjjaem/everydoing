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
파일·메모·사용 기록을\
외부로 보내지 않습니다.

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
    │  드라이브 전체(예: D:\) — ntfs-reader로 MFT 직접 순회
    │  일반 폴더 — 폴더 순회(walkdir)
    │  휴지통·시스템 폴더·점(.) 폴더·Office 임시 파일(~$)은 제외
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

## 설치와 실행 (설치 파일)

- **접속 주소**: 없음. Windows 데스크톱 앱이며 웹 서비스가 아닙니다.
- **실행 환경**: Windows 10/11 (64비트). 화면 표시에 Microsoft Edge WebView2 런타임을 사용하며, 없으면 설치 파일이 내려받아 설치합니다.

1. [Releases](https://github.com/hanjjaem/everydoing/releases/latest)에서 `EveryDoing_<버전>_x64-setup.exe`를 내려받아 실행합니다. 이전 버전이 설치되어 있으면 그 위에 설치되고, 설정·메모·요약은 그대로 남습니다. 0.3.0부터는 앱 안에서 업데이트할 수 있습니다(아래 "업데이트"). 코드 서명이 없어 "Windows의 PC 보호" 창이 뜨면 **추가 정보 → 실행**을 누릅니다.
2. 시작 메뉴에서 **EveryDoing**을 실행하고, 사용자 계정 컨트롤(UAC) 창에서 **예**를 누릅니다. 실행할 때마다 관리자 권한을 요청합니다.
3. 처음 실행하면 설정 패널이 열립니다. **폴더 추가**로 문서가 있는 폴더(예: `C:\Users\(사용자)\Documents`) 또는 드라이브 전체(예: `D:\`)를 고릅니다.
4. **파일 유형**을 확인하고 **저장**을 누르면 스캔이 시작됩니다. 기본 유형은 hwp·hwpx·doc·docx·xls·xlsx·ppt·pptx·pdf입니다.

## 작동 방법

| 하고 싶은 일 | 방법 |
|---|---|
| 그날 다룬 문서 보기 | 날짜 칸의 파일 영역을 누르면 오른쪽에 그날의 파일 목록이 열립니다 |
| 파일·폴더 열기 | 칸의 파일명을 누르고 **파일 열기** 또는 **폴더 열기**를 고릅니다 |
| 다른 달로 이동 | 달력 위의 **월 탭**을 누릅니다. 탭 아래 점은 기록이 있는 달입니다. 해는 **‹ ›**로, 이번 달은 **오늘**로 이동합니다. 키보드 **← →**로도 넘깁니다 |
| 문서를 언제 다뤘는지 찾기 | 사이드바 맨 위 검색에 파일명이나 폴더 이름의 일부를 입력하면 전체 기간에서 찾아 최근 날짜부터 보여줍니다. 결과의 날짜를 누르면 그 달로 이동합니다 |
| 한 줄 메모 | 날짜 칸 아래쪽에 마우스를 올리면 나타나는 **메모 추가**를 누르고 입력한 뒤 Enter를 누릅니다 |
| 새 파일 반영 | 앱을 켤 때 자동으로 다시 스캔합니다. 켜 둔 동안 바뀐 파일은 사이드바 **스캔 폴더**의 **↺**로 반영합니다 |

### 업데이트 (0.3.0~)

- **설정 → 업데이트**의 **지금 확인**을 누르면 새 버전이 있는지 확인합니다.
- 앱을 켤 때 하루에 한 번 자동으로 새 버전을 확인합니다. 원하지 않으면 **설정 → 업데이트**에서 끌 수 있습니다.
- 새 버전이 있으면 사이드바 검색칸 아래에 **새 버전 · 업데이트** 버튼이 나타납니다. 누르면 설치본을 내려받아 서명을 확인한 뒤 설치하고, 앱이 다시 열립니다. 설정·메모·요약은 그대로 남습니다.
- 확인할 때는 GitHub에서 최신 버전 번호가 적힌 파일만 받습니다. 파일명이나 사용 기록은 보내지 않습니다.

### AI 요약 (선택, 기본 꺼짐)

1. **설정**에서 **AI 요약 사용**을 체크하고, 본인의 OpenAI API 키를 입력한 뒤 **저장**합니다. 키는 이 PC의 `%APPDATA%\EveryDoing\config.json`에만 저장됩니다.
2. 사이드바 **AI 요약 생성**(최근 90일 일별 요약) 또는 **월간요약 보기 → AI 월간 총평 생성**을 누릅니다.
3. 요청할 때마다 전송 확인 창이 뜹니다. 날짜와 **파일명만** OpenAI로 전송하며, 파일 내용은 보내지 않습니다. 모델은 현재 gpt-4o-mini로 고정되어 있습니다.

---

## 빠른 시작 (소스로 빌드)

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
  indexer.rs       MFT 순회 · 폴더 순회 · 제외 규칙 · 날짜 변환
  store.rs         index.json 캐시 저장/로드
  summarizer.rs    AI 요약 (OpenAI 호출, 선택 기능)
  bin/dev_web.rs   브라우저 개발 모드 서버 (개발 전용)

static/
  index.html       달력 UI · 사이드바 · 드로어 · 메모 · 설정 패널
  app.js           이벤트 처리 · 달력 렌더 · 검색 · Tauri invoke
  fonts/           Pretendard 글꼴 (앱에 포함, 외부 요청 없음)

tests/             화면 로직 테스트 (node --test "tests/*.test.cjs")
```

개선 방향과 진행 상황은 [개선 계획](docs/개선-계획.md)에 있습니다.

---

## 라이선스

[Business Source License 1.1](LICENSE) — © 2026 한쨈

비프로덕션(개발·학습·테스트) 자유 사용. **2030-06-29** 이후 Apache 2.0 자동 전환.
