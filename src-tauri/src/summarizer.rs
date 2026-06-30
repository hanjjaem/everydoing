use std::collections::BTreeMap;
use serde::Deserialize;

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

pub async fn summarize_dates(
    files_by_date: &BTreeMap<String, Vec<String>>,
    api_key: &str,
) -> BTreeMap<String, String> {
    if api_key.is_empty() || files_by_date.is_empty() {
        return BTreeMap::new();
    }

    // 최근 90일 이내 날짜만 처리
    let cutoff = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days90 = 90u64 * 86400;
        now.saturating_sub(days90)
    };

    let recent: BTreeMap<&String, &Vec<String>> = files_by_date
        .iter()
        .filter(|(date, _)| {
            // "YYYY-MM-DD" 파싱해서 90일 이내인지 확인
            let parts: Vec<&str> = date.split('-').collect();
            if parts.len() != 3 { return false; }
            let (y, m, d) = (
                parts[0].parse::<u64>().unwrap_or(0),
                parts[1].parse::<u64>().unwrap_or(0),
                parts[2].parse::<u64>().unwrap_or(0),
            );
            // 대략적인 unix timestamp 계산 (정밀도 불필요)
            let approx = (y.saturating_sub(1970)) * 365 * 86400
                + m * 30 * 86400 + d * 86400;
            approx >= cutoff
        })
        .collect();

    if recent.is_empty() {
        return BTreeMap::new();
    }

    // 한 번의 요청으로 전체 배치 처리
    let mut lines = String::new();
    for (date, files) in &recent {
        lines.push_str(&format!("{}: {}\n", date, files.join(", ")));
    }

    let prompt = format!(
        "다음은 날짜별 업무 파일명 목록이야. 각 날짜에 어떤 업무를 했는지 15자 이내 한국어로 요약해줘.\n\
         반드시 JSON 형식으로만 응답해 (설명 없이): {{\"날짜\": \"요약\", ...}}\n\n{}",
        lines
    );

    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 2000,
        "temperature": 0.3
    });

    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => { eprintln!("[summarizer] 요청 오류: {}", e); return BTreeMap::new(); }
    };

    let parsed = match resp.json::<ChatResponse>().await {
        Ok(p) => p,
        Err(e) => { eprintln!("[summarizer] 파싱 오류: {}", e); return BTreeMap::new(); }
    };

    let content = match parsed.choices.into_iter().next() {
        Some(c) => c.message.content,
        None => return BTreeMap::new(),
    };

    // JSON 블록 추출 (```json ... ``` 감싸여 있을 수 있음)
    let json_str = {
        let trimmed = content.trim();
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                &trimmed[start..=end]
            } else { trimmed }
        } else { trimmed }
    };

    serde_json::from_str::<BTreeMap<String, String>>(json_str).unwrap_or_default()
}

/// 단일 챗 호출. 성공 시 모델의 응답 텍스트를 그대로 반환.
async fn openai_chat(api_key: &str, prompt: &str, max_tokens: u32) -> Result<String, String> {
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": 0.4
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("요청 오류: {}", e))?;

    let parsed = resp
        .json::<ChatResponse>()
        .await
        .map_err(|e| format!("파싱 오류: {}", e))?;

    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content.trim().to_string())
        .ok_or_else(|| "빈 응답".to_string())
}

/// 한 달치 파일명 목록을 받아 줄글 한국어 총평을 생성.
/// 전송 대상은 파일명뿐이며, 호출 측에서 opt-in 게이트를 통과한 경우에만 실행된다.
pub async fn summarize_month(
    files_by_date: &BTreeMap<String, Vec<String>>,
    ym_label: &str,
    api_key: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("API 키가 없습니다".into());
    }
    if files_by_date.is_empty() {
        return Err("해당 월에 기록이 없습니다".into());
    }

    let mut lines = String::new();
    for (date, files) in files_by_date {
        lines.push_str(&format!("{}: {}\n", date, files.join(", ")));
    }

    let prompt = format!(
        "다음은 {} 한 달 동안의 날짜별 업무 파일명 목록이야. 이 달에 어떤 업무를 했는지 \
         친근하고 따뜻한 말투로 3~4문장 들려줘. 옆에서 말해주듯 '~했어요', '~네요' 같은 \
         부드러운 어미를 쓰고, '~되었습니다 / ~이루어졌습니다' 같은 딱딱한 보고서체는 \
         쓰지 마. 불릿·개조식 말고 줄글로, 파일명을 그대로 나열하지 말고 업무 흐름 위주로 \
         자연스럽게.\n\n{}",
        ym_label, lines
    );

    openai_chat(api_key, &prompt, 600).await
}
