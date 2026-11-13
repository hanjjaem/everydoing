// 서명 빌드(cargo tauri build) 뒤에 실행해, 앱이 읽는 업데이트 정보 파일(latest.json)을 만든다.
// 사용: node scripts/make-latest-json.mjs "이번 버전에서 바뀐 점"
// 결과: src-tauri/target/release/bundle/nsis/latest.json
//       GitHub Release(태그 v<버전>)에 설치본 .exe와 함께 올린다.
import { readFileSync, writeFileSync, existsSync, statSync } from "node:fs";
import { join } from "node:path";

const REPO = "hanjjaem/everydoing";
const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const { version, productName } = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const dir = join(root, "src-tauri", "target", "release", "bundle", "nsis");
const exe = `${productName}_${version}_x64-setup.exe`;
const sig = join(dir, `${exe}.sig`);

for (const f of [join(dir, exe), sig]) {
  if (!existsSync(f)) {
    console.error(`없음: ${f}\n서명 빌드(TAURI_SIGNING_PRIVATE_KEY_PATH·_PASSWORD를 넣고 cargo tauri build)를 먼저 하세요.`);
    process.exit(1);
  }
}
// 서명이 설치본보다 오래됐으면 다른 빌드의 서명일 수 있다.
if (statSync(sig).mtimeMs < statSync(join(dir, exe)).mtimeMs) {
  console.error("서명 파일이 설치본보다 오래됐습니다. 서명 빌드를 다시 하세요.");
  process.exit(1);
}

const manifest = {
  version,
  notes: process.argv[2] || "",
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: readFileSync(sig, "utf8").trim(),
      url: `https://github.com/${REPO}/releases/download/v${version}/${exe}`,
    },
  },
};
const out = join(dir, "latest.json");
writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
console.log(`만듦: ${out}\n버전 ${version} · 설치본 주소 ${manifest.platforms["windows-x86_64"].url}`);
