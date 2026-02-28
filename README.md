<p align="center">
  <img src="app/image/icon.png" width="128" height="128" alt="CatchWord">
</p>

<h1 align="center">CatchWord</h1>

<p align="center">全局划词翻译 + 单词本。双击任意英文单词，即刻弹出翻译。</p>

## 功能

- **全局取词** — 在浏览器、编辑器、PDF、终端等任意应用中双击英文单词即可翻译
- **智能捕获** — 优先使用 Windows UI Automation 读取选中文本；不支持时自动截图 + Windows OCR 识别
- **翻译浮窗** — 鼠标旁弹出浮窗，显示音标、释义、词性、例句
- **自动发音** — 翻译完成后自动播放单词发音
- **单词本** — 查过的单词自动存入本地 JSON 文件，记录查询次数和时间

## 下载

前往 [GitHub Releases](../../releases/latest) 下载最新版本：

- **安装版** — `CatchWord_*_x64-setup.exe`（NSIS 安装包）
- **绿色版** — `CatchWord-portable-*.zip`（解压即用，无需安装）

> 打 tag 即自动构建发布，无需手动操作。

## 截图

> *启动后双击选词即可弹出翻译浮窗*

## 技术栈

| 层      | 技术                                               |
|--------|--------------------------------------------------|
| 框架     | [Tauri v2](https://tauri.app/) (Rust + WebView2) |
| 前端     | HTML / CSS / JS + Vite                           |
| 取词     | Windows UI Automation API                        |
| OCR 兜底 | Windows.Media.Ocr (系统自带，离线可用)                    |
| 鼠标监听   | [rdev](https://crates.io/crates/rdev)            |
| 翻译 API | Google Translate 免费端点 (无需 API Key)               |
| 发音     | 有道词典 TTS                                         |

## 快速开始

### 前置条件

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://www.rust-lang.org/tools/install) >= 1.70
- Windows 10/11 (当前仅支持 Windows)

### 安装与运行

```bash
cd app
npm install
npm run tauri dev
```

### 构建

```bash
cd app
npm run tauri build
```

构建产物在 `app/src-tauri/target/release/bundle/` 目录下。

### 发布

打 tag 并推送，GitHub Actions 会自动构建并创建 Release：

```bash
git tag v0.1.0
git push --tags
```

## 工作原理

```
用户双击单词
  │
  ├─ rdev 检测到双击/拖选事件
  │
  ├─ 策略 1: UIA TextPattern (焦点元素)
  ├─ 策略 2: UIA TextPattern (遍历祖先)
  ├─ 策略 3: UIA TextPattern (遍历子元素)
  ├─ 策略 4: 截图 + Windows OCR (鼠标附近 300×60 区域)
  │
  ├─ 判断是否英文单词
  │
  ├─ 调用 Google Translate API
  │
  └─ 弹出翻译浮窗 + 自动播放发音
```

### 取词兼容性

| 应用类型             | 取词方式 | 支持情况          |
|------------------|------|---------------|
| Chrome / Edge 网页 | UIA  | 支持            |
| VS Code / 编辑器    | UIA  | 支持            |
| PowerShell / 终端  | UIA  | 支持            |
| SumatraPDF       | UIA  | 支持            |
| Adobe Acrobat    | OCR  | 支持            |
| Foxit / WPS PDF  | OCR  | 支持            |
| 其他任意应用           | OCR  | 支持 (需屏幕上可见文字) |

## 项目结构

```
CatchWord/
├── app/
│   ├── src/
│   │   ├── main.js          # 前端：翻译浮窗渲染、窗口自适应
│   │   └── style.css         # 前端：浮窗样式
│   ├── index.html            # 浮窗 HTML
│   ├── package.json
│   └── src-tauri/
│       ├── src/
│       │   ├── lib.rs        # 应用主逻辑、事件循环、系统托盘
│       │   ├── capture.rs    # 取词核心：UIA + OCR
│       │   ├── hook.rs       # 全局鼠标事件监听
│       │   ├── translator.rs # Google Translate API 调用
│       │   ├── types.rs      # 数据结构定义
│       │   └── wordbook.rs   # 单词本读写
│       ├── Cargo.toml
│       └── tauri.conf.json
└── docs/
    └── requirements/         # 需求文档
```

## 配置

- **单词本位置**: `%APPDATA%/com.catchword.app/wordbook.json`
- **系统托盘**: 右键托盘图标可开关取词功能或退出
- **OCR 语言**: 使用 Windows 系统自带英文 OCR (默认已安装)

## 已知限制

- 目前仅支持 Windows 平台
- 仅支持英文 → 中文翻译
- 翻译依赖 Google Translate 免费端点，可能因网络问题不稳定
- OCR 识别准确率取决于屏幕字体清晰度

## License

MIT
