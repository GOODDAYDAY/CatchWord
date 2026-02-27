---
name: capture-safety
description: Text capture safety rules for CatchWord. Consult before ANY change to capture.rs, hook.rs, lib.rs popup handling, or keyboard simulation code.
---

# Capture Safety — 取词安全规则

在修改 CatchWord 的文本捕获逻辑之前，**必须**阅读并遵守以下规则。

## 黄金法则

> **永远不要向非目标窗口发送键盘事件。**

违反此规则会导致：Ctrl+C 变成 Ctrl+F（打开搜索）、Ctrl+E（打开地址栏）、Ctrl+W（关闭标签页）等灾难性行为。

## 事故根因分析

### 事故：Ctrl+C 打开搜索

**因果链：**

```
popup.set_focus() 抢走焦点
→ popup WebView2 成为焦点窗口
→ UIA 失败（GetFocusedElement 返回 popup 而非用户 app）
→ 回退 Ctrl+C
→ enigo 把 Ctrl+C 发给了 popup WebView2
→ WebView2 无选中文本，Ctrl+C 无效
→ Ctrl 键可能因 enigo 时序问题"卡住"
→ 用户后续按键 = Ctrl+按键 = 触发系统快捷键
→ 打开搜索 / 关闭标签 / 其他不可控行为
```

### 事故：点播放按钮也打开搜索

**因果链：**

```
用户点击 popup 上的播放按钮
→ 全局鼠标钩子把此点击判定为"双击选词"
→ PossibleSelection 事件触发
→ popup 隐藏 → 焦点飘到某个窗口
→ capture 运行 → UIA 失败 → Ctrl+C 发给不确定的窗口
→ 打开搜索
```

## 安全检查清单

修改取词相关代码时，逐条检查：

### 1. 焦点管理

- [ ] **绝对不要** 在 popup 上调用 `set_focus()`
- [ ] popup 配置必须有 `alwaysOnTop: true`（不需要焦点也能显示在最前面）
- [ ] 弹出 popup 后，用户 app 必须保持焦点

### 2. 鼠标事件过滤

- [ ] 处理 `PossibleSelection` 之前，**必须先检查**点击坐标是否在 popup 内
- [ ] 在 popup 内的点击 → `continue`，不触发 capture
- [ ] 此检查必须在 `popup.hide()` **之前**执行

### 3. 禁止键盘模拟

- [ ] **不得**引入 `enigo` 依赖
- [ ] **不得**引入 `arboard` 依赖
- [ ] **不得**模拟任何键盘事件（Ctrl+C、Ctrl+V 等）
- [ ] **不得**读写系统剪贴板
- [ ] UIA 失败时 → 返回 None，静默放弃

### 4. UIA 使用

- [ ] 使用 `COINIT_MULTITHREADED`（UIA 客户端必须用 MTA）
- [ ] 不要只试焦点元素，要**遍历祖先和子元素**找 TextPattern
- [ ] `CoUninitialize()` 必须在所有路径上执行（用 closure + 最后统一调用）

### 5. 窗口配置

- [ ] `shadow: false` — 去掉 Windows DWM 系统边框
- [ ] `transparent: true` + `decorations: false`
- [ ] `skipTaskbar: true`
- [ ] 窗口大小由 JS 根据内容动态调整

## 各应用的 UIA 支持情况

| 应用 | TextPattern | 备注 |
|------|-------------|------|
| Chrome/Edge 网页 | 部分支持 | 需遍历祖先找 Document 元素 |
| Chrome/Edge PDF | 实验性 | 不可靠 |
| VS Code / 编辑器 | 支持 | 直接焦点元素可用 |
| PowerShell / 终端 | 支持 | 直接焦点元素可用 |
| Adobe Acrobat | **不支持** | 用 MSAA/IAccessible，需 Ctrl+C 兜底 |
| Foxit Reader | **不支持** | UIA 实现不完整且极慢 |
| WPS Office | **不支持** | 无文档化的 UIA 支持 |
| SumatraPDF | **支持** | 有完整的 ITextProvider 实现 |

## 捕获策略优先级

```
1. UIA TextPattern（焦点元素）          ← 最可靠，无副作用
2. UIA TextPattern（遍历祖先元素）      ← 浏览器必需
3. UIA TextPattern（遍历子元素）        ← 某些特殊控件
4. 返回 None（静默放弃）               ← UIA 全部失败时
```

## 绝对禁止

> **禁止使用 enigo / arboard / 模拟键盘 / Ctrl+C 剪贴板方案。**
>
> 任何形式的键盘模拟都可能向错误窗口发送不可控的快捷键，
> 导致打开搜索、关闭标签页等灾难性行为。
> 即使加了"安全检查"也无法覆盖所有边界情况。
>
> 如果 UIA 不支持某个应用，正确做法是**静默放弃**，
> 而不是用不可控的手段尝试获取文本。

## 测试场景

每次修改后，必须在以下场景测试：

1. **Chrome 网页**双击选词 → 应该走 UIA 策略 2
2. **VS Code / 终端**双击选词 → 应该走 UIA 策略 1
3. **PDF 阅读器**双击选词 → 可能走 Ctrl+C 兜底
4. **点击 popup 播放按钮** → 不应触发任何 capture
5. **popup 显示时双击其他单词** → 应正常工作，不打开搜索
6. **popup 显示时点击空白处** → popup 隐藏，不触发 capture
