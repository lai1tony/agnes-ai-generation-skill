use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tokio::time::{sleep, Instant};

const BASE_URL: &str = "https://apihub.agnes-ai.com";
const TEXT_MODEL: &str = "agnes-2.0-flash";
const IMAGE_MODEL: &str = "agnes-image-2.1-flash";
const VIDEO_MODEL: &str = "agnes-video-v2.0";
const KEYRING_SERVICE: &str = "agnes-ai-studio";
const KEYRING_USER: &str = "agnes-api-key";

#[derive(Debug, Error)]
enum AppError {
    #[error("Agnes API Key 未配置，请先在设置页保存 API Key。")]
    MissingApiKey,
    #[error("HTTP {status} {path}: {body}")]
    Http {
        status: u16,
        path: String,
        body: String,
    },
    #[error("{0}")]
    Validation(String),
    #[error("请求 Agnes API 失败: {0}")]
    Request(String),
    #[error("解析响应失败: {0}")]
    Json(String),
    #[error("保存文件失败: {0}")]
    Io(String),
    #[error("系统安全存储失败: {0}")]
    Keyring(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&redact_secret(&self.to_string()))
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextRequest {
    prompt: String,
    system: Option<String>,
    temperature: f32,
    top_p: Option<f32>,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextResult {
    content: Option<String>,
    raw: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageRequest {
    prompt: String,
    size: Option<String>,
    images: Vec<String>,
    translate_prompt: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageResult {
    kind: String,
    prompt_used: String,
    translated_prompt: Option<String>,
    urls: Vec<String>,
    raw: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoRequest {
    prompt: String,
    images: Vec<String>,
    mode: Option<String>,
    resolution: Option<String>,
    aspect_ratio: Option<String>,
    duration_seconds: Option<u32>,
    height: Option<u32>,
    width: Option<u32>,
    num_frames: Option<u32>,
    frame_rate: Option<f32>,
    num_inference_steps: Option<u32>,
    seed: Option<i64>,
    negative_prompt: Option<String>,
    translate_prompt: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoGetRequest {
    task_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoResult {
    kind: String,
    task_id: Option<String>,
    video_id: Option<String>,
    prompt_used: Option<String>,
    translated_prompt: Option<String>,
    status: Option<String>,
    urls: Vec<String>,
    raw: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveMediaRequest {
    url: String,
    path: String,
}

#[derive(Clone)]
struct AgnesClient {
    base_url: String,
    http: Client,
}

impl Default for AgnesClient {
    fn default() -> Self {
        Self {
            base_url: BASE_URL.to_string(),
            http: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
        }
    }
}

impl AgnesClient {
    async fn post_json(&self, path: &str, api_key: &str, payload: &Value) -> AppResult<Value> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .http
            .post(url)
            .bearer_auth(api_key)
            .json(payload)
            .send()
            .await
            .map_err(|err| AppError::Request(err.to_string()))?;
        response_to_json(response, path).await
    }

    async fn get_json(&self, path: &str, api_key: &str) -> AppResult<Value> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .http
            .get(url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|err| AppError::Request(err.to_string()))?;
        response_to_json(response, path).await
    }
}

async fn response_to_json(response: reqwest::Response, path: &str) -> AppResult<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| AppError::Request(err.to_string()))?;
    if !status.is_success() {
        return Err(AppError::Http {
            status: status.as_u16(),
            path: path.to_string(),
            body: redact_secret(&body),
        });
    }
    serde_json::from_str(&body).map_err(|err| AppError::Json(err.to_string()))
}

fn keyring_entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|err| AppError::Keyring(err.to_string()))
}

fn get_api_key() -> AppResult<String> {
    let key = keyring_entry()?
        .get_password()
        .map_err(|_| AppError::MissingApiKey)?;
    if key.trim().is_empty() {
        Err(AppError::MissingApiKey)
    } else {
        Ok(key)
    }
}

fn set_api_key(value: &str) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("API Key 不能为空。".to_string()));
    }
    keyring_entry()?
        .set_password(trimmed)
        .map_err(|err| AppError::Keyring(err.to_string()))
}

fn delete_stored_key() -> AppResult<()> {
    match keyring_entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(AppError::Keyring(err.to_string())),
    }
}

fn redact_secret(text: &str) -> String {
    let mut redacted = text.to_string();
    for marker in ["Bearer ", "AGNES_API_KEY", "AGNES_API_TOKEN", "APIHUB_AGNES_API_KEY"] {
        redacted = redacted.replace(marker, "[redacted]");
    }
    redacted
}

fn ensure_prompt(prompt: &str) -> AppResult<()> {
    if prompt.trim().is_empty() {
        Err(AppError::Validation("Prompt 不能为空。".to_string()))
    } else {
        Ok(())
    }
}

fn needs_english_translation(prompt: &str) -> bool {
    prompt.chars().any(|ch| ch as u32 > 127)
}

fn validate_size(value: Option<&str>) -> AppResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    let parts: Vec<&str> = value.split('x').collect();
    if parts.len() != 2 {
        return Err(AppError::Validation(format!("图片尺寸无效：{value}，请使用 1024x768 这样的格式。")));
    }
    let valid = parts.iter().all(|part| {
        !part.is_empty()
            && !part.starts_with('0')
            && part.chars().all(|ch| ch.is_ascii_digit())
            && part.parse::<u32>().map(|number| number > 0).unwrap_or(false)
    });
    if valid {
        Ok(())
    } else {
        Err(AppError::Validation(format!("图片尺寸无效：{value}，请使用 1024x768 这样的格式。")))
    }
}

fn validate_video_args(args: &VideoRequest) -> AppResult<()> {
    if let Some(resolution) = normalized_text(args.resolution.as_deref()) {
        if !matches!(resolution, "480p" | "720p" | "1080p") {
            return Err(AppError::Validation(
                "视频分辨率只能是 480p、720p 或 1080p。".to_string(),
            ));
        }
    }
    if let Some(aspect_ratio) = normalized_text(args.aspect_ratio.as_deref()) {
        if !matches!(aspect_ratio, "16:9" | "9:16" | "1:1" | "4:3" | "3:4") {
            return Err(AppError::Validation(
                "视频宽高比只能是 16:9、9:16、1:1、4:3 或 3:4。".to_string(),
            ));
        }
    }
    if let Some(duration) = args.duration_seconds {
        if video_frames_for_duration(duration).is_none() {
            return Err(AppError::Validation(
                "视频长度只能是 3、5、10 或 18 秒。".to_string(),
            ));
        }
    }
    if let Some(num_frames) = args.num_frames {
        if num_frames > 441 || (num_frames - 1) % 8 != 0 {
            return Err(AppError::Validation(
                "视频帧数必须 <= 441 且满足 8n + 1，例如 81 或 121。".to_string(),
            ));
        }
    }
    if let Some(frame_rate) = args.frame_rate {
        if !(1.0..=60.0).contains(&frame_rate) {
            return Err(AppError::Validation("视频帧率必须在 1-60 之间。".to_string()));
        }
    }
    for (label, value) in [("高度", args.height), ("宽度", args.width)] {
        if matches!(value, Some(0)) {
            return Err(AppError::Validation(format!("视频{label}必须是正整数。")));
        }
    }
    if let Some(mode) = &args.mode {
        if mode != "ti2vid" && mode != "keyframes" {
            return Err(AppError::Validation("视频模式只能是 ti2vid 或 keyframes。".to_string()));
        }
    }
    let image_count = args.images.iter().filter(|value| !value.trim().is_empty()).count();
    if image_count > 1 && args.mode.as_deref() == Some("ti2vid") {
        return Err(AppError::Validation(
            "多张输入图片不能使用 ti2vid，请选择 keyframes 或使用自动模式。".to_string(),
        ));
    }
    Ok(())
}

fn normalized_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn video_dimensions(resolution: &str, aspect_ratio: &str) -> Option<(u32, u32)> {
    match (resolution, aspect_ratio) {
        ("480p", "16:9") => Some((848, 480)),
        ("480p", "9:16") => Some((480, 848)),
        ("480p", "1:1") => Some((480, 480)),
        ("480p", "4:3") => Some((640, 480)),
        ("480p", "3:4") => Some((480, 640)),
        ("720p", "16:9") => Some((1280, 720)),
        ("720p", "9:16") => Some((720, 1280)),
        ("720p", "1:1") => Some((720, 720)),
        ("720p", "4:3") => Some((960, 720)),
        ("720p", "3:4") => Some((720, 960)),
        ("1080p", "16:9") => Some((1920, 1080)),
        ("1080p", "9:16") => Some((1080, 1920)),
        ("1080p", "1:1") => Some((1080, 1080)),
        ("1080p", "4:3") => Some((1440, 1080)),
        ("1080p", "3:4") => Some((1080, 1440)),
        _ => None,
    }
}

fn video_frames_for_duration(duration_seconds: u32) -> Option<u32> {
    match duration_seconds {
        3 => Some(81),
        5 => Some(121),
        10 => Some(241),
        18 => Some(441),
        _ => None,
    }
}

async fn translate_prompt(client: &AgnesClient, api_key: &str, prompt: &str) -> AppResult<String> {
    let payload = json!({
        "model": TEXT_MODEL,
        "messages": [
            {
                "role": "system",
                "content": "Translate the user's image/video generation prompt into fluent English. Preserve all concrete visual details, style words, camera motion, lighting, composition constraints, and negative instructions. Return only the English prompt."
            },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0,
        "max_tokens": 800
    });
    let data = client.post_json("/v1/chat/completions", api_key, &payload).await?;
    let translated = data
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| AppError::Validation("提示词翻译失败：Agnes 未返回可用文本。".to_string()))?;
    Ok(translated.to_string())
}

async fn prepare_generation_prompt(
    client: &AgnesClient,
    api_key: &str,
    prompt: &str,
    translate: bool,
) -> AppResult<(String, Option<String>)> {
    ensure_prompt(prompt)?;
    if translate && needs_english_translation(prompt) {
        let translated = translate_prompt(client, api_key, prompt).await?;
        Ok((translated.clone(), Some(translated)))
    } else {
        Ok((prompt.trim().to_string(), None))
    }
}

fn extract_text_content(data: &Value) -> Option<String> {
    data.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn extract_image_urls(data: &Value) -> Vec<String> {
    let mut urls = Vec::new();
    push_string_field(data, "url", &mut urls);
    push_string_field(data, "image_url", &mut urls);
    if let Some(items) = data.get("data").and_then(Value::as_array) {
        for item in items {
            push_string_field(item, "url", &mut urls);
            push_string_field(item, "image_url", &mut urls);
        }
    }
    dedupe(urls)
}

fn extract_video_urls(data: &Value) -> Vec<String> {
    let mut urls = Vec::new();
    for key in ["video_url", "url", "remixed_from_video_id"] {
        if let Some(value) = data.get(key).and_then(Value::as_str) {
            if value.starts_with("http://") || value.starts_with("https://") {
                urls.push(value.to_string());
            }
        }
    }
    if let Some(items) = data.get("data").and_then(Value::as_array) {
        for item in items {
            urls.extend(extract_video_urls(item));
        }
    }
    dedupe(urls)
}

fn push_string_field(data: &Value, key: &str, urls: &mut Vec<String>) {
    if let Some(value) = data.get(key).and_then(Value::as_str) {
        urls.push(value.to_string());
    }
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn build_video_payload(args: &VideoRequest, prompt: &str) -> AppResult<Value> {
    validate_video_args(args)?;
    let mut payload = Map::new();
    payload.insert("model".to_string(), json!(VIDEO_MODEL));
    payload.insert("prompt".to_string(), json!(prompt));
    let resolution = normalized_text(args.resolution.as_deref());
    let aspect_ratio = normalized_text(args.aspect_ratio.as_deref());
    if resolution.is_some() || aspect_ratio.is_some() {
        let (width, height) = video_dimensions(resolution.unwrap_or("720p"), aspect_ratio.unwrap_or("16:9"))
            .ok_or_else(|| AppError::Validation("视频分辨率或宽高比无效。".to_string()))?;
        payload.insert("width".to_string(), json!(width));
        payload.insert("height".to_string(), json!(height));
    } else {
        insert_optional(&mut payload, "height", args.height);
        insert_optional(&mut payload, "width", args.width);
    }
    if let Some(duration) = args.duration_seconds {
        let frames = video_frames_for_duration(duration)
            .ok_or_else(|| AppError::Validation("视频长度只能是 3、5、10 或 18 秒。".to_string()))?;
        payload.insert("num_frames".to_string(), json!(frames));
        payload.insert("frame_rate".to_string(), json!(24.0));
    } else {
        insert_optional(&mut payload, "num_frames", args.num_frames);
        insert_optional(&mut payload, "frame_rate", args.frame_rate);
    }
    insert_optional(&mut payload, "num_inference_steps", args.num_inference_steps);
    insert_optional(&mut payload, "seed", args.seed);
    if let Some(value) = args.negative_prompt.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        payload.insert("negative_prompt".to_string(), json!(value));
    }
    let images: Vec<String> = args
        .images
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let explicit_mode = args.mode.as_deref().filter(|value| !value.is_empty());
    let effective_mode = explicit_mode.or_else(|| (images.len() > 1).then_some("keyframes"));
    if let Some(mode) = effective_mode {
        payload.insert("mode".to_string(), json!(mode));
    }
    if !images.is_empty() {
        if images.len() == 1 && effective_mode != Some("keyframes") {
            payload.insert("image".to_string(), json!(images[0]));
        } else {
            let mut extra = Map::new();
            extra.insert("image".to_string(), json!(images));
            if let Some(mode) = effective_mode {
                extra.insert("mode".to_string(), json!(mode));
            }
            payload.insert("extra_body".to_string(), Value::Object(extra));
        }
    }
    Ok(Value::Object(payload))
}

fn build_image_payload(size: Option<&str>, images: &[String], prompt: &str) -> Value {
    let mut payload = Map::new();
    payload.insert("model".to_string(), json!(IMAGE_MODEL));
    payload.insert("prompt".to_string(), json!(prompt));
    if let Some(size) = size.map(str::trim).filter(|value| !value.is_empty()) {
        payload.insert("size".to_string(), json!(size));
    }
    let mut extra = Map::new();
    extra.insert("response_format".to_string(), json!("url"));
    if !images.is_empty() {
        extra.insert("image".to_string(), json!(images));
    }
    payload.insert("extra_body".to_string(), Value::Object(extra));
    Value::Object(payload)
}

fn insert_optional<T: Serialize>(payload: &mut Map<String, Value>, key: &str, value: Option<T>) {
    if let Some(value) = value {
        payload.insert(key.to_string(), json!(value));
    }
}

fn first_string(data: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| data.get(*key).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn status_string(data: &Value) -> Option<String> {
    data.get("status")
        .map(|value| value.to_string().trim_matches('"').to_string())
}

fn video_query_path(task_id: &str) -> AppResult<String> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(AppError::Validation("视频任务 ID 不能为空。".to_string()));
    }
    if task_id.starts_with("video_") {
        Ok(format!("/agnesapi?video_id={task_id}&model_name={VIDEO_MODEL}"))
    } else {
        Ok(format!("/v1/videos/{task_id}"))
    }
}

#[tauri::command]
fn save_api_key(api_key: String) -> AppResult<()> {
    set_api_key(&api_key)
}

#[tauri::command]
fn check_api_key() -> AppResult<bool> {
    Ok(get_api_key().is_ok())
}

#[tauri::command]
fn delete_api_key() -> AppResult<()> {
    delete_stored_key()
}

#[tauri::command]
async fn generate_text(request: TextRequest, state: tauri::State<'_, AgnesClient>) -> AppResult<TextResult> {
    ensure_prompt(&request.prompt)?;
    let api_key = get_api_key()?;
    let mut messages = Vec::new();
    if let Some(system) = request.system.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        messages.push(json!({ "role": "system", "content": system }));
    }
    messages.push(json!({ "role": "user", "content": request.prompt.trim() }));
    let mut payload = Map::new();
    payload.insert("model".to_string(), json!(TEXT_MODEL));
    payload.insert("messages".to_string(), json!(messages));
    payload.insert("temperature".to_string(), json!(request.temperature));
    payload.insert("max_tokens".to_string(), json!(request.max_tokens));
    insert_optional(&mut payload, "top_p", request.top_p);
    let raw = state.post_json("/v1/chat/completions", &api_key, &Value::Object(payload)).await?;
    Ok(TextResult {
        content: extract_text_content(&raw),
        raw,
    })
}

#[tauri::command]
async fn generate_image(request: ImageRequest, state: tauri::State<'_, AgnesClient>) -> AppResult<ImageResult> {
    ensure_prompt(&request.prompt)?;
    validate_size(request.size.as_deref())?;
    let api_key = get_api_key()?;
    let (prompt_used, translated_prompt) =
        prepare_generation_prompt(&state, &api_key, &request.prompt, request.translate_prompt).await?;
    let images: Vec<String> = request
        .images
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let payload = build_image_payload(request.size.as_deref(), &images, &prompt_used);
    let raw = state.post_json("/v1/images/generations", &api_key, &payload).await?;
    Ok(ImageResult {
        kind: if images.is_empty() { "text-to-image" } else { "image-to-image" }.to_string(),
        prompt_used,
        translated_prompt,
        urls: extract_image_urls(&raw),
        raw,
    })
}

#[tauri::command]
async fn create_video_task(request: VideoRequest, state: tauri::State<'_, AgnesClient>) -> AppResult<VideoResult> {
    ensure_prompt(&request.prompt)?;
    validate_video_args(&request)?;
    let api_key = get_api_key()?;
    let (prompt_used, translated_prompt) =
        prepare_generation_prompt(&state, &api_key, &request.prompt, request.translate_prompt).await?;
    let payload = build_video_payload(&request, &prompt_used)?;
    let raw = state.post_json("/v1/videos", &api_key, &payload).await?;
    let task_id = first_string(&raw, &["id", "task_id"]);
    let video_id = first_string(&raw, &["video_id"]);
    Ok(VideoResult {
        kind: "video-task".to_string(),
        task_id,
        video_id,
        prompt_used: Some(prompt_used),
        translated_prompt,
        status: status_string(&raw),
        urls: extract_video_urls(&raw),
        raw,
    })
}

#[tauri::command]
async fn get_video_task(request: VideoGetRequest, state: tauri::State<'_, AgnesClient>) -> AppResult<VideoResult> {
    get_video_task_inner(&state, &request.task_id).await
}

async fn get_video_task_inner(client: &AgnesClient, task_id: &str) -> AppResult<VideoResult> {
    let task_id = task_id.trim();
    let path = video_query_path(task_id)?;
    let api_key = get_api_key()?;
    let raw = client.get_json(&path, &api_key).await?;
    Ok(VideoResult {
        kind: "video-result".to_string(),
        task_id: first_string(&raw, &["id", "task_id"]).or_else(|| (!task_id.starts_with("video_")).then(|| task_id.to_string())),
        video_id: first_string(&raw, &["video_id"]).or_else(|| task_id.starts_with("video_").then(|| task_id.to_string())),
        prompt_used: None,
        translated_prompt: None,
        status: status_string(&raw),
        urls: extract_video_urls(&raw),
        raw,
    })
}

#[tauri::command]
async fn poll_video_task(
    request: VideoGetRequest,
    timeout_seconds: u64,
    interval_seconds: u64,
    state: tauri::State<'_, AgnesClient>,
) -> AppResult<VideoResult> {
    let deadline = Instant::now() + Duration::from_secs(timeout_seconds.max(1));
    let interval = Duration::from_secs(interval_seconds.max(1));
    loop {
        let result = get_video_task_inner(&state, &request.task_id).await?;
        let status = result.status.clone().unwrap_or_default().to_lowercase();
        if status == "completed" || status == "failed" || Instant::now() >= deadline {
            return Ok(result);
        }
        sleep(interval).await;
    }
}

#[tauri::command]
async fn save_remote_media(request: SaveMediaRequest) -> AppResult<String> {
    if !(request.url.starts_with("http://") || request.url.starts_with("https://")) {
        return Err(AppError::Validation("只能保存 http/https 媒体链接。".to_string()));
    }
    let path = PathBuf::from(request.path);
    let bytes = Client::new()
        .get(&request.url)
        .send()
        .await
        .map_err(|err| AppError::Request(err.to_string()))?
        .bytes()
        .await
        .map_err(|err| AppError::Request(err.to_string()))?;
    std::fs::write(&path, bytes).map_err(|err| AppError::Io(err.to_string()))?;
    Ok(path.to_string_lossy().to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AgnesClient::default())
        .invoke_handler(tauri::generate_handler![
            save_api_key,
            check_api_key,
            delete_api_key,
            generate_text,
            generate_image,
            create_video_task,
            get_video_task,
            poll_video_task,
            save_remote_media
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agnes AI Studio");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_image_size() {
        assert!(validate_size(Some("1024x768")).is_ok());
        assert!(validate_size(Some("0x768")).is_err());
        assert!(validate_size(Some("1024*768")).is_err());
    }

    #[test]
    fn validates_video_frames_and_rate() {
        let valid = video_request(vec![], None);
        assert!(validate_video_args(&valid).is_ok());
        assert!(validate_video_args(&VideoRequest { num_frames: Some(120), ..valid }).is_err());
    }

    fn video_request(images: Vec<&str>, mode: Option<&str>) -> VideoRequest {
        VideoRequest {
            prompt: "test".into(),
            images: images.into_iter().map(ToString::to_string).collect(),
            mode: mode.map(ToString::to_string),
            resolution: None,
            aspect_ratio: None,
            duration_seconds: None,
            height: Some(768),
            width: Some(1152),
            num_frames: Some(121),
            frame_rate: Some(24.0),
            num_inference_steps: Some(28),
            seed: Some(42),
            negative_prompt: Some("low quality".into()),
            translate_prompt: true,
        }
    }

    #[test]
    fn builds_text_to_video_payload() {
        let payload = build_video_payload(&video_request(vec![], None), "prompt").expect("payload");
        assert_eq!(payload["model"], VIDEO_MODEL);
        assert_eq!(payload["prompt"], "prompt");
        assert_eq!(payload["width"], 1152);
        assert_eq!(payload["height"], 768);
        assert_eq!(payload["num_frames"], 121);
        assert_eq!(payload["frame_rate"], 24.0);
        assert_eq!(payload["num_inference_steps"], 28);
        assert_eq!(payload["seed"], 42);
        assert_eq!(payload["negative_prompt"], "low quality");
        assert!(payload.get("image").is_none());
        assert!(payload.get("extra_body").is_none());
    }

    #[test]
    fn builds_normalized_video_payload() {
        let request = VideoRequest {
            resolution: Some("720p".into()),
            aspect_ratio: Some("16:9".into()),
            duration_seconds: Some(5),
            width: Some(480),
            height: Some(480),
            num_frames: Some(81),
            frame_rate: Some(12.0),
            ..video_request(vec![], None)
        };
        let payload = build_video_payload(&request, "prompt").expect("payload");
        assert_eq!(payload["width"], 1280);
        assert_eq!(payload["height"], 720);
        assert_eq!(payload["num_frames"], 121);
        assert_eq!(payload["frame_rate"], 24.0);
    }

    #[test]
    fn rejects_invalid_normalized_video_options() {
        assert!(validate_video_args(&VideoRequest {
            resolution: Some("2k".into()),
            ..video_request(vec![], None)
        })
        .is_err());
        assert!(validate_video_args(&VideoRequest {
            aspect_ratio: Some("21:9".into()),
            ..video_request(vec![], None)
        })
        .is_err());
        assert!(validate_video_args(&VideoRequest {
            duration_seconds: Some(4),
            ..video_request(vec![], None)
        })
        .is_err());
    }

    #[test]
    fn builds_video_query_paths() {
        assert_eq!(
            video_query_path("video_abc").expect("video path"),
            "/agnesapi?video_id=video_abc&model_name=agnes-video-v2.0"
        );
        assert_eq!(
            video_query_path("task_abc").expect("task path"),
            "/v1/videos/task_abc"
        );
        assert!(video_query_path(" ").is_err());
    }

    #[test]
    fn builds_single_image_to_video_payload() {
        let payload = build_video_payload(&video_request(vec!["https://example.com/a.png"], None), "prompt")
            .expect("payload");
        assert_eq!(payload["image"], "https://example.com/a.png");
        assert!(payload.get("extra_body").is_none());
    }

    #[test]
    fn builds_multi_image_video_payload() {
        let payload = build_video_payload(
            &video_request(vec!["https://example.com/a.png", "https://example.com/b.png"], None),
            "prompt",
        )
        .expect("payload");
        assert!(payload.get("image").is_none());
        assert_eq!(
            payload["extra_body"]["image"],
            json!(["https://example.com/a.png", "https://example.com/b.png"])
        );
        assert_eq!(payload["mode"], "keyframes");
        assert_eq!(payload["extra_body"]["mode"], "keyframes");
    }

    #[test]
    fn rejects_multi_image_ti2vid_payload() {
        let request = video_request(vec!["https://example.com/a.png", "https://example.com/b.png"], Some("ti2vid"));
        assert!(validate_video_args(&request).is_err());
    }

    #[test]
    fn builds_keyframe_video_payload() {
        let payload = build_video_payload(
            &video_request(vec!["https://example.com/a.png", "https://example.com/b.png"], Some("keyframes")),
            "prompt",
        )
        .expect("payload");
        assert_eq!(payload["mode"], "keyframes");
        assert_eq!(
            payload["extra_body"]["image"],
            json!(["https://example.com/a.png", "https://example.com/b.png"])
        );
        assert_eq!(payload["extra_body"]["mode"], "keyframes");
    }

    #[test]
    fn keyframe_mode_with_one_image_still_uses_extra_body() {
        let payload = build_video_payload(&video_request(vec!["https://example.com/a.png"], Some("keyframes")), "prompt")
            .expect("payload");
        assert!(payload.get("image").is_none());
        assert_eq!(payload["extra_body"]["image"], json!(["https://example.com/a.png"]));
        assert_eq!(payload["extra_body"]["mode"], "keyframes");
    }

    #[test]
    fn detects_non_ascii_prompt() {
        assert!(needs_english_translation("未来城市"));
        assert!(!needs_english_translation("future city"));
    }

    #[test]
    fn extracts_urls_from_responses() {
        let image = json!({
            "data": [
                { "url": "https://example.com/a.png" },
                { "image_url": "https://example.com/b.png" }
            ]
        });
        assert_eq!(extract_image_urls(&image).len(), 2);
        let video = json!({
            "data": [{ "video_url": "https://example.com/a.mp4" }],
            "remixed_from_video_id": "https://example.com/b.mp4"
        });
        assert_eq!(extract_video_urls(&video).len(), 2);
    }

    #[test]
    fn builds_text_to_image_payload() {
        let payload = build_image_payload(Some("1024x768"), &[], "prompt");
        assert_eq!(payload["model"], IMAGE_MODEL);
        assert_eq!(payload["prompt"], "prompt");
        assert_eq!(payload["size"], "1024x768");
        assert_eq!(payload["extra_body"]["response_format"], "url");
        assert!(payload["extra_body"].get("image").is_none());
    }

    #[test]
    fn builds_image_edit_payload() {
        let images = vec!["https://example.com/a.png".to_string(), "https://example.com/b.png".to_string()];
        let payload = build_image_payload(Some("1024x768"), &images, "edit prompt");
        assert_eq!(payload["model"], IMAGE_MODEL);
        assert_eq!(payload["prompt"], "edit prompt");
        assert_eq!(payload["extra_body"]["response_format"], "url");
        assert_eq!(payload["extra_body"]["image"], json!(images));
    }

    #[test]
    #[ignore = "writes to the user's OS credential store; run explicitly with AGNES_KEYRING_TEST_KEY"]
    fn saves_and_reads_api_key_from_system_keyring() {
        let key = std::env::var("AGNES_KEYRING_TEST_KEY").expect("AGNES_KEYRING_TEST_KEY must be set");

        save_api_key(key.clone()).expect("save api key");

        assert!(check_api_key().expect("check api key"));
        assert_eq!(get_api_key().expect("read api key"), key.trim());
    }

    #[tokio::test]
    async fn save_remote_media_downloads_to_file() {
        use httpmock::Method::GET;
        use httpmock::MockServer;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/media.bin");
            then.status(200).body("media bytes");
        });
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("media.bin");

        let saved = save_remote_media(SaveMediaRequest {
            url: format!("{}/media.bin", server.base_url()),
            path: path.to_string_lossy().to_string(),
        })
        .await
        .expect("saved media");

        assert_eq!(saved, path.to_string_lossy());
        assert_eq!(std::fs::read(path).expect("saved file"), b"media bytes");
    }

    #[tokio::test]
    async fn agnes_client_handles_mock_success_and_error() {
        use httpmock::Method::{GET, POST};
        use httpmock::MockServer;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200).json_body(json!({
                "choices": [{ "message": { "content": "ok" } }]
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/v1/videos/bad");
            then.status(500).body("{\"error\":\"provider failed\"}");
        });

        let client = AgnesClient {
            base_url: server.base_url(),
            http: Client::new(),
        };
        let ok = client
            .post_json("/v1/chat/completions", "test-key", &json!({}))
            .await
            .expect("mock success");
        assert_eq!(extract_text_content(&ok).as_deref(), Some("ok"));

        let err = client
            .get_json("/v1/videos/bad", "test-key")
            .await
            .expect_err("mock error");
        assert!(err.to_string().contains("HTTP 500"));
    }

    #[tokio::test]
    #[ignore = "calls the live Agnes video API and waits for generation"]
    async fn live_agnes_video_task_completes() {
        let api_key = get_api_key().expect("saved Agnes API key");
        let client = AgnesClient::default();
        let request = VideoRequest {
            prompt: "A calm studio shot of a small glass cube rotating slowly on a white background".into(),
            images: vec![],
            mode: None,
            resolution: Some("480p".into()),
            aspect_ratio: Some("16:9".into()),
            duration_seconds: Some(3),
            height: None,
            width: None,
            num_frames: None,
            frame_rate: None,
            num_inference_steps: None,
            seed: None,
            negative_prompt: None,
            translate_prompt: false,
        };
        let payload = build_video_payload(&request, &request.prompt).expect("video payload");
        let mut create_attempt = 0;
        let created = loop {
            match client.post_json("/v1/videos", &api_key, &payload).await {
                Ok(data) => break data,
                Err(AppError::Http { status: 429, .. }) if create_attempt == 0 => {
                    create_attempt += 1;
                    sleep(Duration::from_secs(70)).await;
                }
                Err(err) => panic!("create video task failed: {err}"),
            }
        };
        let task_id = first_string(&created, &["video_id", "id", "task_id"]).expect("video id");
        eprintln!("created video task: {task_id}");

        for _ in 0..60 {
            let result = get_video_task_inner(&client, &task_id).await.expect("query video task");
            let status = result.status.clone().unwrap_or_default();
            eprintln!(
                "video {task_id}: status={status} seconds={:?} size={:?}",
                result.raw.get("seconds"),
                result.raw.get("size")
            );
            if status.eq_ignore_ascii_case("completed") {
                assert!(!result.urls.is_empty(), "completed video should include a URL");
                eprintln!("video url: {}", result.urls[0]);
                return;
            }
            assert_ne!(status.to_lowercase(), "failed", "video task failed: {}", result.raw);
            sleep(Duration::from_secs(15)).await;
        }
        panic!("timed out waiting for video task {task_id}");
    }
}
