# KeyScroll

> 键盘就是你的滚轮。  
> Your keyboard, now your scroll wheel.

---

**KeyScroll** 是一款轻量级的 Windows 后台工具，让你用键盘快捷键代替鼠标滚轮滚动页面。特别适合以下场景：

- 😤 界面没有滚动条，鼠标滚轮滚了半天到底
- ⚡ 鼠标中键自动滚动在部分应用中不可用
- 💪 需要长时间浏览长文档 / 网页，手指不想一直滚轮

**核心功能**：按下 `Ctrl+↑` 或 `Ctrl+↓` 即可持续滚动，松开即停。长按支持加速度加速。

---

## 快速开始

### 方式一：AutoHotkey 原型（即用）

> 适合想**今天就用上**的人。

从 [Releases](https://github.com/caiyilian/KeyScroll/releases) 下载 `keyscroll-ahk.exe`，双击运行，然后按 `Ctrl+↑/↓` 即可。

或在本地用 AutoHotkey v2 编译源码：

```powershell
# 需要 AutoHotkey v2 + Ahk2Exe
Ahk2Exe.exe /in src-ahk/keyscroll.ahk /out keyscroll.exe
```

### 方式二：Rust 原生构建

> 适合想要单文件 < 1MB、零依赖原生体验的人。

```powershell
cargo build --release
.\target\release\keyscroll.exe
```

---

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+↑` | 向上持续滚动 |
| `Ctrl+↓` | 向下持续滚动 |
| `Ctrl+Shift+↑` | 向左持续滚动（水平） |
| `Ctrl+Shift+↓` | 向右持续滚动（水平） |

> 所有快捷键均可在 `config.toml` 中自定义（Phase 3+）。

---

## 项目状态

KeyScroll 目前处于**开发规划阶段**。详见 [开发计划](docs/dev-plan.md)。

### 路线图

| Phase | 内容 | 状态 |
|-------|------|------|
| Phase 0 | AutoHotkey 快速原型 | 🟡 规划中 |
| Phase 1 | Rust 核心 + 全局热键 | ⬜ 待开始 |
| Phase 2 | 滚动加速度与平滑停止 | ⬜ 待开始 |
| Phase 3 | 配置文件驱动 | ⬜ 待开始 |
| Phase 4 | 系统托盘 + 视觉反馈 | ⬜ 待开始 |
| Phase 5 | 开机自启 + 安装包 | ⬜ 待开始 |
| Phase 6 | 高级特性（每应用配置等） | ⬜ 待开始 |

---

## 技术栈

- **Phase 0**: AutoHotkey v2
- **Phase 1+**: Rust + [`windows-rs`](https://github.com/microsoft/windows-rs)
  - 全局热键：`RegisterHotKey` API
  - 滚动模拟：`SendInput` + `MOUSEEVENTF_WHEEL`
  - 配置：TOML
  - 打包：WiX Toolset (MSI)

---

## License

MIT
