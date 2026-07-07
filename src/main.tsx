import React, { FormEvent, useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Download,
  Eraser,
  FileText,
  Film,
  History,
  Image as ImageIcon,
  KeyRound,
  Link,
  Loader2,
  Play,
  Save,
  Settings,
  Trash2,
} from "lucide-react";
import "./styles.css";

type TabId = "text" | "image" | "video" | "history" | "settings";

type TauriWindow = Window & {
  __TAURI_INTERNALS__?: unknown;
};

type SettingsState = {
  imageSize: string;
  videoResolution: string;
  videoAspectRatio: string;
  videoDurationSeconds: string;
  translatePrompt: boolean;
};

type HistoryItem = {
  id: string;
  type: "text" | "image" | "video";
  title: string;
  prompt: string;
  translatedPrompt?: string | null;
  status?: string | null;
  urls: string[];
  createdAt: string;
};

type TextResult = {
  content?: string | null;
  raw: unknown;
};

type ImageResult = {
  kind: string;
  promptUsed: string;
  translatedPrompt?: string | null;
  urls: string[];
  raw: unknown;
};

type VideoResult = {
  kind: string;
  taskId?: string | null;
  videoId?: string | null;
  promptUsed?: string | null;
  translatedPrompt?: string | null;
  status?: string | null;
  urls: string[];
  raw: unknown;
};

const defaultSettings: SettingsState = {
  imageSize: "1024x768",
  videoResolution: "720p",
  videoAspectRatio: "16:9",
  videoDurationSeconds: "5",
  translatePrompt: true,
};

const videoResolutions = ["480p", "720p", "1080p"];
const videoAspectRatios = ["16:9", "9:16", "1:1", "4:3", "3:4"];
const videoDurations = ["3", "5", "10", "18"];

const tabs: Array<{ id: TabId; label: string; icon: React.ElementType }> = [
  { id: "text", label: "文本", icon: FileText },
  { id: "image", label: "图片", icon: ImageIcon },
  { id: "video", label: "视频", icon: Film },
  { id: "history", label: "历史", icon: History },
  { id: "settings", label: "设置", icon: Settings },
];

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as TauriWindow);
}

function formatError(value: unknown) {
  if (!isTauriRuntime()) {
    return "当前页面运行在浏览器预览中，无法访问 Tauri 桌面后端。请在 Agnes AI Studio 桌面窗口中使用保存密钥、生成和保存文件功能。";
  }

  if (typeof value === "string") {
    return value;
  }

  if (value instanceof Error) {
    return value.message;
  }

  if (value && typeof value === "object") {
    try {
      const json = JSON.stringify(value);
      if (json && json !== "{}") {
        return json;
      }
    } catch {
      // Fall through to the generic message.
    }
  }

  return "操作失败。请查看开发控制台中的错误详情。";
}

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function normalizeSettings(value: Partial<SettingsState>): SettingsState {
  return {
    ...defaultSettings,
    ...value,
    videoResolution: videoResolutions.includes(value.videoResolution || "")
      ? value.videoResolution!
      : defaultSettings.videoResolution,
    videoAspectRatio: videoAspectRatios.includes(value.videoAspectRatio || "")
      ? value.videoAspectRatio!
      : defaultSettings.videoAspectRatio,
    videoDurationSeconds: videoDurations.includes(value.videoDurationSeconds || "")
      ? value.videoDurationSeconds!
      : defaultSettings.videoDurationSeconds,
  };
}

function compactJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

function splitLines(value: string): string[] {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function optionalNumber(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const number = Number(trimmed);
  return Number.isFinite(number) ? number : null;
}

function makeHistoryItem(
  type: HistoryItem["type"],
  title: string,
  prompt: string,
  urls: string[],
  translatedPrompt?: string | null,
  status?: string | null,
): HistoryItem {
  return {
    id: crypto.randomUUID(),
    type,
    title,
    prompt,
    translatedPrompt,
    status,
    urls,
    createdAt: new Date().toISOString(),
  };
}

function App() {
  const [tab, setTab] = useState<TabId>("text");
  const [settings, setSettings] = useState<SettingsState>(() =>
    normalizeSettings(loadJson("agnes.settings", defaultSettings)),
  );
  const [history, setHistory] = useState<HistoryItem[]>(() => loadJson("agnes.history", []));
  const [hasApiKey, setHasApiKey] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("check_api_key")
      .then(setHasApiKey)
      .catch(() => setHasApiKey(false));
  }, []);

  useEffect(() => {
    localStorage.setItem("agnes.settings", JSON.stringify(settings));
  }, [settings]);

  useEffect(() => {
    localStorage.setItem("agnes.history", JSON.stringify(history.slice(0, 80)));
  }, [history]);

  const addHistory = (item: HistoryItem) => {
    setHistory((items) => [item, ...items].slice(0, 80));
  };

  const notify = (message: string) => {
    setToast(message);
    window.setTimeout(() => setToast(null), 2200);
  };

  const handleError = (value: unknown) => {
    setError(formatError(value));
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">A</div>
          <div>
            <h1>Agnes AI Studio</h1>
            <p>{hasApiKey ? "API Key 已配置" : "请先配置 API Key"}</p>
          </div>
        </div>
        <nav className="nav">
          {tabs.map(({ id, label, icon: Icon }) => (
            <button key={id} className={tab === id ? "active" : ""} onClick={() => setTab(id)}>
              <Icon size={18} />
              <span>{label}</span>
            </button>
          ))}
        </nav>
      </aside>

      <main className="workspace">
        <TopBar hasApiKey={hasApiKey} onSettings={() => setTab("settings")} />
        {tab === "text" && (
          <TextPanel onError={handleError} onHistory={addHistory} notify={notify} />
        )}
        {tab === "image" && (
          <ImagePanel
            settings={settings}
            onError={handleError}
            onHistory={addHistory}
            notify={notify}
          />
        )}
        {tab === "video" && (
          <VideoPanel
            settings={settings}
            onError={handleError}
            onHistory={addHistory}
            notify={notify}
          />
        )}
        {tab === "history" && (
          <HistoryPanel
            history={history}
            setHistory={setHistory}
            notify={notify}
            onError={handleError}
          />
        )}
        {tab === "settings" && (
          <SettingsPanel
            settings={settings}
            setSettings={setSettings}
            hasApiKey={hasApiKey}
            setHasApiKey={setHasApiKey}
            notify={notify}
            onError={handleError}
          />
        )}
      </main>

      {toast && <div className="toast">{toast}</div>}
      {error && (
        <div className="modal-backdrop" onClick={() => setError(null)}>
          <section className="modal" onClick={(event) => event.stopPropagation()}>
            <h2>错误详情</h2>
            <pre>{error}</pre>
            <button className="primary" onClick={() => setError(null)}>
              关闭
            </button>
          </section>
        </div>
      )}
    </div>
  );
}

function TopBar({ hasApiKey, onSettings }: { hasApiKey: boolean; onSettings: () => void }) {
  return (
    <header className="topbar">
      <div>
        <h2>生成工作台</h2>
        <p>文本、图片与视频任务共用 Agnes 官方 API。</p>
      </div>
      <button className={hasApiKey ? "status ok" : "status warn"} onClick={onSettings}>
        <KeyRound size={16} />
        <span>{hasApiKey ? "密钥可用" : "配置密钥"}</span>
      </button>
    </header>
  );
}

function TextPanel({
  onError,
  onHistory,
  notify,
}: {
  onError: (value: unknown) => void;
  onHistory: (item: HistoryItem) => void;
  notify: (message: string) => void;
}) {
  const [prompt, setPrompt] = useState("");
  const [system, setSystem] = useState("");
  const [temperature, setTemperature] = useState("0.7");
  const [maxTokens, setMaxTokens] = useState("1024");
  const [result, setResult] = useState<TextResult | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setResult(null);
    try {
      const data = await invoke<TextResult>("generate_text", {
        request: {
          prompt,
          system: system || null,
          temperature: Number(temperature),
          maxTokens: Number(maxTokens),
          topP: null,
        },
      });
      setResult(data);
      onHistory(makeHistoryItem("text", "文本生成", prompt, [], null, "completed"));
      notify("文本生成完成");
    } catch (err) {
      onError(err);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="page-grid two">
      <form className="panel" onSubmit={submit}>
        <PanelTitle title="文本生成" />
        <label>
          系统指令
          <textarea value={system} onChange={(event) => setSystem(event.target.value)} rows={3} />
        </label>
        <label>
          Prompt
          <textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} rows={9} required />
        </label>
        <div className="field-row">
          <label>
            Temperature
            <input value={temperature} onChange={(event) => setTemperature(event.target.value)} />
          </label>
          <label>
            Max tokens
            <input value={maxTokens} onChange={(event) => setMaxTokens(event.target.value)} />
          </label>
        </div>
        <button className="primary" disabled={busy}>
          {busy ? <Loader2 className="spin" size={18} /> : <Play size={18} />}
          <span>生成文本</span>
        </button>
      </form>
      <ResultPanel title="文本结果">
        {result ? (
          <>
            <div className="text-output">{result.content || "Agnes 未返回文本内容。"}</div>
            <Details value={result.raw} />
          </>
        ) : (
          <EmptyState label="结果会显示在这里。" />
        )}
      </ResultPanel>
    </section>
  );
}

function ImagePanel({
  settings,
  onError,
  onHistory,
  notify,
}: {
  settings: SettingsState;
  onError: (value: unknown) => void;
  onHistory: (item: HistoryItem) => void;
  notify: (message: string) => void;
}) {
  const [prompt, setPrompt] = useState("");
  const [size, setSize] = useState(settings.imageSize);
  const [images, setImages] = useState("");
  const [result, setResult] = useState<ImageResult | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setResult(null);
    try {
      const data = await invoke<ImageResult>("generate_image", {
        request: {
          prompt,
          size,
          images: splitLines(images),
          translatePrompt: settings.translatePrompt,
        },
      });
      setResult(data);
      onHistory(makeHistoryItem("image", data.kind, prompt, data.urls, data.translatedPrompt, "completed"));
      notify("图片生成完成");
    } catch (err) {
      onError(err);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="page-grid two">
      <form className="panel" onSubmit={submit}>
        <PanelTitle title="图片生成" />
        <label>
          Prompt
          <textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} rows={8} required />
        </label>
        <div className="field-row">
          <label>
            尺寸
            <input value={size} onChange={(event) => setSize(event.target.value)} placeholder="1024x768" />
          </label>
        </div>
        <label>
          输入图片 URL
          <textarea
            value={images}
            onChange={(event) => setImages(event.target.value)}
            rows={5}
            placeholder="每行一个 URL，留空为文生图"
          />
        </label>
        <button className="primary" disabled={busy}>
          {busy ? <Loader2 className="spin" size={18} /> : <ImageIcon size={18} />}
          <span>生成图片</span>
        </button>
      </form>
      <MediaResult title="图片结果" result={result} notify={notify} onError={onError} />
    </section>
  );
}

function VideoPanel({
  settings,
  onError,
  onHistory,
  notify,
}: {
  settings: SettingsState;
  onError: (value: unknown) => void;
  onHistory: (item: HistoryItem) => void;
  notify: (message: string) => void;
}) {
  const [prompt, setPrompt] = useState("");
  const [images, setImages] = useState("");
  const [mode, setMode] = useState("");
  const [resolution, setResolution] = useState(settings.videoResolution);
  const [aspectRatio, setAspectRatio] = useState(settings.videoAspectRatio);
  const [durationSeconds, setDurationSeconds] = useState(settings.videoDurationSeconds);
  const [seed, setSeed] = useState("");
  const [negativePrompt, setNegativePrompt] = useState("");
  const [taskId, setTaskId] = useState("");
  const [result, setResult] = useState<VideoResult | null>(null);
  const [busy, setBusy] = useState(false);

  const createTask = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    try {
      const data = await invoke<VideoResult>("create_video_task", {
        request: {
          prompt,
          images: splitLines(images),
          mode: mode || null,
          resolution,
          aspectRatio,
          durationSeconds: optionalNumber(durationSeconds),
          width: null,
          height: null,
          numFrames: null,
          frameRate: null,
          numInferenceSteps: null,
          seed: optionalNumber(seed),
          negativePrompt: negativePrompt || null,
          translatePrompt: settings.translatePrompt,
        },
      });
      setResult(data);
      setTaskId(data.videoId || data.taskId || "");
      onHistory(
        makeHistoryItem("video", "视频任务", prompt, data.urls, data.translatedPrompt, data.status || "created"),
      );
      notify("视频任务已创建");
    } catch (err) {
      onError(err);
    } finally {
      setBusy(false);
    }
  };

  const getTask = async (poll = false) => {
    setBusy(true);
    try {
      const command = poll ? "poll_video_task" : "get_video_task";
      const payload = poll
        ? { request: { taskId }, timeoutSeconds: 900, intervalSeconds: 10 }
        : { request: { taskId } };
      const data = await invoke<VideoResult>(command, payload);
      setResult(data);
      onHistory(makeHistoryItem("video", "视频查询", taskId, data.urls, null, data.status));
      notify(poll ? "视频轮询结束" : "视频状态已更新");
    } catch (err) {
      onError(err);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="page-grid two">
      <form className="panel" onSubmit={createTask}>
        <PanelTitle title="视频生成" />
        <label>
          Prompt
          <textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} rows={6} required />
        </label>
        <label>
          输入图片 URL
          <textarea
            value={images}
            onChange={(event) => setImages(event.target.value)}
            rows={4}
            placeholder="每行一个 URL，留空为文生视频"
          />
        </label>
        <div className="field-row">
          <label>
            模式
            <select value={mode} onChange={(event) => setMode(event.target.value)}>
              <option value="">自动</option>
              <option value="ti2vid">ti2vid</option>
              <option value="keyframes">keyframes</option>
            </select>
          </label>
          <label>
            Seed
            <input value={seed} onChange={(event) => setSeed(event.target.value)} />
          </label>
        </div>
        <div className="field-row">
          <label>
            分辨率
            <select value={resolution} onChange={(event) => setResolution(event.target.value)}>
              {videoResolutions.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
          <label>
            宽高比
            <select value={aspectRatio} onChange={(event) => setAspectRatio(event.target.value)}>
              {videoAspectRatios.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
        </div>
        <label>
          视频长度
          <select value={durationSeconds} onChange={(event) => setDurationSeconds(event.target.value)}>
            {videoDurations.map((value) => (
              <option key={value} value={value}>
                约 {value} 秒
              </option>
            ))}
          </select>
        </label>
        <label>
          负面提示词
          <input value={negativePrompt} onChange={(event) => setNegativePrompt(event.target.value)} />
        </label>
        <button className="primary" disabled={busy}>
          {busy ? <Loader2 className="spin" size={18} /> : <Film size={18} />}
          <span>创建视频任务</span>
        </button>
        <div className="task-row">
          <input value={taskId} onChange={(event) => setTaskId(event.target.value)} placeholder="视频 ID / 任务 ID" />
          <button type="button" disabled={busy || !taskId.trim()} onClick={() => getTask(false)}>
            查询
          </button>
          <button type="button" disabled={busy || !taskId.trim()} onClick={() => getTask(true)}>
            轮询
          </button>
        </div>
      </form>
      <MediaResult title="视频结果" result={result} notify={notify} onError={onError} />
    </section>
  );
}

function HistoryPanel({
  history,
  setHistory,
  notify,
  onError,
}: {
  history: HistoryItem[];
  setHistory: React.Dispatch<React.SetStateAction<HistoryItem[]>>;
  notify: (message: string) => void;
  onError: (value: unknown) => void;
}) {
  return (
    <section className="panel full">
      <div className="section-head">
        <PanelTitle title="本地历史" />
        <button className="danger" onClick={() => setHistory([])}>
          <Trash2 size={16} />
          <span>清空</span>
        </button>
      </div>
      {history.length === 0 ? (
        <EmptyState label="生成记录会保存在这里，仅包含任务元数据和 URL。" />
      ) : (
        <div className="history-list">
          {history.map((item) => (
            <article className="history-item" key={item.id}>
              <div>
                <strong>{item.title}</strong>
                <span>{new Date(item.createdAt).toLocaleString()}</span>
              </div>
              <p>{item.prompt}</p>
              {item.translatedPrompt && <p className="muted">英文提示词：{item.translatedPrompt}</p>}
              {item.status && <span className="pill">{item.status}</span>}
              <UrlList urls={item.urls} notify={notify} onError={onError} />
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function SettingsPanel({
  settings,
  setSettings,
  hasApiKey,
  setHasApiKey,
  notify,
  onError,
}: {
  settings: SettingsState;
  setSettings: React.Dispatch<React.SetStateAction<SettingsState>>;
  hasApiKey: boolean;
  setHasApiKey: (value: boolean) => void;
  notify: (message: string) => void;
  onError: (value: unknown) => void;
}) {
  const [apiKey, setApiKey] = useState("");

  const saveKey = async () => {
    try {
      await invoke("save_api_key", { apiKey });
      setApiKey("");
      setHasApiKey(true);
      notify("API Key 已保存到系统安全存储");
    } catch (err) {
      onError(err);
    }
  };

  const deleteKey = async () => {
    try {
      await invoke("delete_api_key");
      setHasApiKey(false);
      notify("API Key 已删除");
    } catch (err) {
      onError(err);
    }
  };

  return (
    <section className="page-grid two">
      <div className="panel">
        <PanelTitle title="API Key" />
        <p className="hint">密钥保存在系统安全存储中，前端不会读取明文。</p>
        <label>
          Agnes API Key
          <input
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
            type="password"
            placeholder={hasApiKey ? "已配置，可输入新值覆盖" : "请输入 API Key"}
          />
        </label>
        <div className="button-row">
          <button className="primary" onClick={saveKey} disabled={!apiKey.trim()}>
            <Save size={16} />
            <span>保存</span>
          </button>
          <button className="danger" onClick={deleteKey}>
            <Eraser size={16} />
            <span>删除</span>
          </button>
        </div>
      </div>
      <div className="panel">
        <PanelTitle title="默认参数" />
        <label>
          图片尺寸
          <input
            value={settings.imageSize}
            onChange={(event) => setSettings((state) => ({ ...state, imageSize: event.target.value }))}
          />
        </label>
        <div className="field-row">
          <label>
            视频分辨率
            <select
              value={settings.videoResolution}
              onChange={(event) => setSettings((state) => ({ ...state, videoResolution: event.target.value }))}
            >
              {videoResolutions.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
          <label>
            视频宽高比
            <select
              value={settings.videoAspectRatio}
              onChange={(event) => setSettings((state) => ({ ...state, videoAspectRatio: event.target.value }))}
            >
              {videoAspectRatios.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
        </div>
        <label>
          视频长度
          <select
            value={settings.videoDurationSeconds}
            onChange={(event) => setSettings((state) => ({ ...state, videoDurationSeconds: event.target.value }))}
          >
            {videoDurations.map((value) => (
              <option key={value} value={value}>
                约 {value} 秒
              </option>
            ))}
          </select>
        </label>
        <label className="toggle">
          <input
            type="checkbox"
            checked={settings.translatePrompt}
            onChange={(event) => setSettings((state) => ({ ...state, translatePrompt: event.target.checked }))}
          />
          <span>自动将非英文图片/视频提示词翻译为英文</span>
        </label>
      </div>
    </section>
  );
}

function MediaResult({
  title,
  result,
  notify,
  onError,
}: {
  title: string;
  result: ImageResult | VideoResult | null;
  notify: (message: string) => void;
  onError: (value: unknown) => void;
}) {
  const urls = result?.urls || [];
  return (
    <ResultPanel title={title}>
      {result ? (
        <>
          {"status" in result && result.status && <span className="pill">{result.status}</span>}
          {"translatedPrompt" in result && result.translatedPrompt && (
            <p className="hint">英文提示词：{result.translatedPrompt}</p>
          )}
          <div className="media-grid">
            {urls.map((url) => (
              <MediaPreview key={url} url={url} notify={notify} onError={onError} />
            ))}
          </div>
          {urls.length === 0 && <EmptyState label="当前响应里没有可直接预览的媒体 URL。" />}
          <UrlList urls={urls} notify={notify} onError={onError} />
          <Details value={result.raw} />
        </>
      ) : (
        <EmptyState label="生成结果会显示在这里。" />
      )}
    </ResultPanel>
  );
}

function MediaPreview({
  url,
  notify,
  onError,
}: {
  url: string;
  notify: (message: string) => void;
  onError: (value: unknown) => void;
}) {
  const isVideo = /\.(mp4|mov|webm)(\?|$)/i.test(url);
  return (
    <figure className="media-preview">
      {isVideo ? <video src={url} controls /> : <img src={url} alt="Agnes 生成结果" />}
      <figcaption>
        <button title="打开链接" onClick={() => openUrl(url)}>
          <Link size={15} />
        </button>
        <SaveButton url={url} notify={notify} onError={onError} />
      </figcaption>
    </figure>
  );
}

function SaveButton({
  url,
  notify,
  onError,
}: {
  url: string;
  notify: (message: string) => void;
  onError: (value: unknown) => void;
}) {
  const saveMedia = async () => {
    try {
      const extension = /\.(png|jpe?g|webp|gif|mp4|mov|webm)(\?|$)/i.exec(url)?.[1] || "bin";
      const path = await save({
        defaultPath: `agnes-result.${extension}`,
      });
      if (!path) return;
      await invoke("save_remote_media", { request: { url, path } });
      notify("媒体已保存");
    } catch (err) {
      onError(err);
    }
  };
  return (
    <button title="保存到本地" onClick={saveMedia}>
      <Download size={15} />
    </button>
  );
}

function UrlList({
  urls,
  notify,
  onError,
}: {
  urls: string[];
  notify: (message: string) => void;
  onError: (value: unknown) => void;
}) {
  if (urls.length === 0) return null;
  return (
    <div className="url-list">
      {urls.map((url) => (
        <div key={url}>
          <span>{url}</span>
          <button
            onClick={() => {
              navigator.clipboard.writeText(url).then(() => notify("URL 已复制")).catch(onError);
            }}
          >
            复制
          </button>
        </div>
      ))}
    </div>
  );
}

function Details({ value }: { value: unknown }) {
  const [open, setOpen] = useState(false);
  const text = useMemo(() => compactJson(value), [value]);
  return (
    <details open={open} onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary>{open ? "收起原始响应" : "查看原始响应"}</summary>
      <pre className="raw-json">{text}</pre>
    </details>
  );
}

function ResultPanel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="panel result-panel">
      <PanelTitle title={title} />
      {children}
    </section>
  );
}

function PanelTitle({ title }: { title: string }) {
  return <h3 className="panel-title">{title}</h3>;
}

function EmptyState({ label }: { label: string }) {
  return <div className="empty-state">{label}</div>;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
