# Agnes AI Generation Skill

中文 | [English](#english)

用于 Codex / Agent Skills 的 Agnes AI 生成技能，封装 Agnes 官方文本、图片、视频 API。安装后，你可以让 AI 直接调用 Agnes 模型完成文生图、图生图、文生视频、图生视频、多图视频、关键帧动画等操作。

官网与 API 平台：[https://platform.agnes-ai.com/](https://platform.agnes-ai.com/)

## 功能

- 文本生成：`agnes-2.0-flash`
- 流式文本响应
- OpenAI 兼容的工具调用请求结构
- 文生图：`agnes-image-2.1-flash`
- 图生图 / 图片编辑：`agnes-image-2.1-flash`
- 高信息密度图片生成
- 文本转视频：`agnes-video-v2.0`
- 图像转视频：`agnes-video-v2.0`
- 多图像视频生成
- 关键帧动画
- 基于提示词的运动与场景控制
- 电影感视觉输出
- 异步视频任务创建
- 轮询式视频结果检索
- 基于 seed 的可重复生成
- 自动将非英文图片/视频提示词翻译为英文提示词，提高 Agnes 视频生成稳定性

## 快速开始

### 1. 申请 Agnes API Key

1. 打开 Agnes API 平台：[https://platform.agnes-ai.com/](https://platform.agnes-ai.com/)
2. 注册或登录账号。
3. 在平台中申请 / 创建 API Key。
4. 拿到 API Key 后，可以在可信任的当前 AI 会话里发送给 AI，或配置为本机环境变量。

安全提醒：不要把 API Key 写入 Git 仓库、README、截图或公开聊天记录。

### 2. 安装本 Skill

使用 `npx skills add` 安装：

```powershell
npx skills add Yacey/agnes-ai-generation-skill --skill agnes-ai-generation --agent codex --copy -g -y
```

安装后，当你要求 AI 使用 Agnes 生成图片或视频时，AI 会自动触发本 skill。

### 3. 配置 API Key

临时配置当前 PowerShell 会话：

```powershell
$env:AGNES_API_KEY="YOUR_API_KEY"
```

Windows 用户级持久配置：

```powershell
[Environment]::SetEnvironmentVariable("AGNES_API_KEY", "YOUR_API_KEY", "User")
```

脚本也会识别以下变量名：

- `AGNES_API_KEY`
- `AGNES_API_TOKEN`
- `APIHUB_AGNES_API_KEY`

### 4. 开始使用

你可以直接对 AI 说：

```text
使用 Agnes 帮我生成一张高信息密度的未来城市图片。
```

或：

```text
使用 Agnes 把这张图片生成一段电影感视频。
```

如果你已经把 API Key 发送给当前 AI，AI 可以配置环境变量并调用本 skill。之后即可解锁生图、生视频等相关操作。

## 命令示例

文本生成：

```powershell
python scripts/agnes_api.py text --prompt "Write a concise product tagline for an AI assistant."
```

文生图：

```powershell
python scripts/agnes_api.py image --prompt "A luminous floating city above a misty canyon at sunrise, cinematic realism" --size 1024x768
```

中文提示词文生图，脚本会先自动翻译成英文：

```powershell
python scripts/agnes_api.py image --prompt "一座高信息密度的未来城市集市，拥挤人群，飞行汽车，全息招牌，电影感写实风格"
```

图生图：

```powershell
python scripts/agnes_api.py image --prompt "Turn the scene into a rainy cyberpunk night while preserving composition" --image https://example.com/input.png
```

文生视频：

```powershell
python scripts/agnes_api.py video --prompt "A cinematic shot of a cat walking on the beach at sunset" --poll
```

图生视频：

```powershell
python scripts/agnes_api.py video --prompt "Animate subtle camera movement and natural lighting" --image https://example.com/image.png --poll
```

多图 / 关键帧视频：

```powershell
python scripts/agnes_api.py video --prompt "Create a smooth cinematic transition between the two keyframes" --image https://example.com/a.png --image https://example.com/b.png --mode keyframes --poll
```

查询视频任务：

```powershell
python scripts/agnes_api.py video-get task_123456
```

运行测试：

```powershell
python scripts/agnes_api.py smoke-test
```

单独测试某个视频能力，避免一次创建太多视频任务：

```powershell
python scripts/agnes_api.py smoke-test --video-case text-to-video
```

可选的视频测试项：

- `text-to-video`
- `image-to-video`
- `multi-image`
- `keyframes`

## 提示词语言策略

Agnes 视频生成使用英文提示词更稳定。因此本 skill 的脚本默认会检测图片/视频提示词中的非英文字符，并先调用 `agnes-2.0-flash` 翻译成英文生成提示词，再调用图片或视频 API。

翻译时会保留：

- 主体
- 场景
- 风格
- 光照
- 构图
- 镜头运动
- 动作描述
- 负面提示词或约束

如果你确实想跳过自动翻译，可以加：

```powershell
--no-translate-prompt
```

示例：

```powershell
python scripts/agnes_api.py video --prompt "中文提示词" --no-translate-prompt
```

## 测试状态

已通过真实 API 测试：

- 基础文本生成
- 流式文本
- 工具调用请求结构
- 文生图
- 图生图
- 高信息密度文生图
- 中文提示词自动翻译后文生图

已部分验证：

- 视频任务创建
- 视频任务查询接口可达

尚未完整端到端验证：

- 每一种视频模式都成功返回最终 `video_url`

说明：第一次真实文生视频任务在查询时返回 Agnes 服务端 `division by zero` 错误。因此视频模式在 skill 中已支持，但在某个任务达到 `completed` 并返回 `video_url` 前，不应标记为全部端到端通过。

## 仓库结构

```text
.
├── SKILL.md
├── README.md
├── LICENSE
├── agents/
│   └── openai.yaml
├── references/
│   └── api.md
└── scripts/
    └── agnes_api.py
```

## 许可证

MIT License. See [LICENSE](LICENSE).

---

## English

Agnes AI Generation Skill is a Codex / Agent Skills package for calling Agnes AI text, image, and video generation APIs. After installation, AI agents can use Agnes models for text generation, text-to-image, image-to-image, text-to-video, image-to-video, multi-image video, and keyframe animation workflows.

Official platform: [https://platform.agnes-ai.com/](https://platform.agnes-ai.com/)

## Features

- Text generation with `agnes-2.0-flash`
- Streaming text responses
- OpenAI-compatible tool-calling request shape
- Text-to-image with `agnes-image-2.1-flash`
- Image-to-image editing with `agnes-image-2.1-flash`
- High-information-density image generation
- Text-to-video with `agnes-video-v2.0`
- Image-to-video with `agnes-video-v2.0`
- Multi-image video generation
- Keyframe animation
- Prompt-based motion and scene control
- Cinematic video output
- Asynchronous video task creation
- Polling-based video result retrieval
- Seed-based reproducibility
- Automatic English prompt translation for non-English image/video prompts

## Quick Start

### 1. Apply for an Agnes API Key

1. Open [https://platform.agnes-ai.com/](https://platform.agnes-ai.com/).
2. Register or sign in.
3. Create an API key from the platform.
4. Provide the API key to the AI in a trusted current session, or configure it as a local environment variable.

Do not commit API keys to Git, README files, screenshots, or public chat logs.

### 2. Install This Skill

```powershell
npx skills add Yacey/agnes-ai-generation-skill --skill agnes-ai-generation --agent codex --copy -g -y
```

### 3. Configure the API Key

For the current PowerShell session:

```powershell
$env:AGNES_API_KEY="YOUR_API_KEY"
```

For persistent Windows user-level configuration:

```powershell
[Environment]::SetEnvironmentVariable("AGNES_API_KEY", "YOUR_API_KEY", "User")
```

The script also accepts:

- `AGNES_API_KEY`
- `AGNES_API_TOKEN`
- `APIHUB_AGNES_API_KEY`

### 4. Use the Skill

Ask your AI agent:

```text
Use Agnes to generate a high-information-density futuristic city image.
```

Or:

```text
Use Agnes to turn this image into a cinematic video.
```

## Prompt Language

English prompts are more stable for Agnes video generation. For image and video calls, this skill automatically translates non-English prompts to English before sending them to the Agnes image/video APIs. It preserves subjects, scene details, style, lighting, composition, camera movement, motion, and constraints.

To disable automatic translation:

```powershell
python scripts/agnes_api.py video --prompt "non-English prompt" --no-translate-prompt
```

## License

MIT License. See [LICENSE](LICENSE).
