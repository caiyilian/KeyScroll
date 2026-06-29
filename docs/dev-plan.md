# KeyScroll 开发计划 — 键盘快捷键替代鼠标滚轮

## 项目概述

**问题**：界面无滚动条或页面极长时，鼠标滚轮效率低下，鼠标中键自动滚动在某些界面无效。

**方案**：一个常驻后台的轻量工具，注册全局热键（Ctrl+↑/↓），按下时持续发送滚动事件，松开即停止。让你在任意界面都能用键盘舒舒服服地滚屏。

**目标平台**：Windows（当前环境 win32）

**技术路线**：分两条线推进 —— **Phase 0 先用 AutoHotkey 快速出活儿**，后续用 **Rust + windows-rs 构建原生单二进制程序**，无任何运行时依赖。

---

## Phase 0 — 快速原型（AutoHotkey v2）

> 让你**今天就能用上**。不纠结架构，只求功能跑通。

### 具体任务

1. 编写 AHK v2 脚本 `keyscroll.ahk`:
   - 注册 `Ctrl+↑` → 持续发送 `{WheelUp}`
   - 注册 `Ctrl+↓` → 持续发送 `{WheelDown}`
   - 按下时以固定间隔（~50ms）循环发送，松开停止
2. 编译为独立 EXE（`Ahk2Exe`），免去用户安装 AHK
3. 测试：在浏览器、VSCode、Windows 资源管理器等场景验证

### 验收标准

- [ ] `Ctrl+↑` 持续向上滚动，松开停止
- [ ] `Ctrl+↓` 持续向下滚动，松开停止
- [ ] 滚动速率可接受（不做精确调速，固定间隔即可）
- [ ] 编译后的 EXE 无需额外依赖，双击即用
- [ ] 不与常见应用（游戏、IDE）的已有快捷键冲突

### 工作量

约 20-30 行代码，一小时内完成。

---

## Phase 1 — Rust 项目脚手架 + 核心热键注册

> 正式项目的起点。用 Rust 重写核心逻辑，注册系统级全局热键，用 `SendInput` API 模拟滚动事件。

### 具体任务

1. `cargo init keyscroll`，添加依赖：
   - `windows` crate（或 `windows-sys`）：访问 `RegisterHotKey`、`SendInput`
   - `tray-icon` / `windows-tray`：待 Phase 4
2. 实现 Windows 消息循环（`GetMessage` / `PeekMessage` 分发）
3. 用 `RegisterHotKey(MOD_CONTROL, VK_UP)` / `VK_DOWN` 注册全局热键
4. 热键触发时，后台线程以固定间隔调用 `SendInput` 发送 `MOUSEEVENTF_WHEEL` 事件
5. 松开热键时停止发送

### 验收标准

- [ ] 编译通过，生成单 EXE（< 1MB）
- [ ] 在任何窗口中按下 Ctrl+↑ 持续向上滚动，松开停止
- [ ] 在任何窗口中按下 Ctrl+↓ 持续向下滚动，松开停止
- [ ] 不与其他使用 Ctrl+Arrow 的应用程序（如 Excel 单元格导航）冲突（用户可配置前的已知限制）
- [ ] 不触发 Windows UAC 弹窗（无需管理员权限）

### 工作量

一个 Rust 新人约 2-3 天，有经验者 1 天。核心约 120-200 行代码。

### 关键 API 速查

```rust
// 注册热键
RegisterHotKey(nullptr, id, MOD_CONTROL, VK_UP);
RegisterHotKey(nullptr, id, MOD_CONTROL, VK_DOWN);

// 消息循环
while (GetMessage(&msg, nullptr, 0, 0)) {
    if (msg.message == WM_HOTKEY) { /* 触发滚动线程 */ }
}

// 模拟滚轮
INPUT input = {0};
input.type = INPUT_MOUSE;
input.mi.dwFlags = MOUSEEVENTF_WHEEL;
input.mi.mouseData = WHEEL_DELTA; // 120 = 一个刻度
SendInput(1, &input, sizeof(INPUT));
```

---

## Phase 2 — 滚动行为精细化

> 基础的固定间隔滚动不够自然。这一阶段让滚动行为接近真实鼠标滚轮手感。

### 具体任务

1. **加速度曲线**：按下持续时间越长，滚动步长 / 频率逐渐增大
   - 前 500ms：慢速（120 ticks / 80ms 间隔）
   - 500ms–2s：中速（240 ticks / 40ms 间隔）
   - 2s+：高速（480 ticks / 20ms 间隔）
2. **平滑停止**：松开后并非立即归零，而是用 100ms 做线性衰减
3. **可调方向**：支持 `Ctrl+Shift+↑/↓` 切换滚动方向（vh — 部分界面用水平滚动）
4. **低延迟响应**：按下到第一次滚动事件发出的延迟 < 30ms

### 验收标准

- [ ] 长按 5 秒后，滚动速度明显快于刚按下时（加速度生效）
- [ ] 松开快捷键后，滚动不是急停，而是约 100ms 内逐渐停止
- [ ] `Ctrl+Shift+↑/↓` 可以触发水平滚动（`MOUSEEVENTF_HWHEEL`）
- [ ] 按下瞬间即有响应，无明显延迟感

### 工作量

约 100-150 行代码，1-2 天。

---

## Phase 3 — 配置文件 + 用户自定义

> 硬编码的热键和速度不适用于所有人。增加 JSON/TOML 配置文件，让用户无需重新编译即可调整一切。

### 具体任务

1. 选择配置格式（推荐 TOML，对非开发者友好）
2. 确定配置文件路径：`%APPDATA%/keyscroll/config.toml` 及本地 `config.toml`（以本地优先）
3. 配置文件默认内容：

```toml
[hotkeys]
scroll_up = "Ctrl+Up"
scroll_down = "Ctrl+Down"
scroll_up_horizontal = "Ctrl+Shift+Up"   # 可选
scroll_down_horizontal = "Ctrl+Shift+Down"

[scroll]
initial_delay_ms = 80     # 初始滚动间隔
min_delay_ms = 16         # 最快间隔 (~60fps)
acceleration_start_ms = 500   # 多久开始加速
acceleration_max_ms = 3000    # 多久达到最大速度
step_size = 120           # WHEEL_DELTA 步长
horizontal_step_size = 120

[behavior]
stop_on_key_release = true
smooth_stop_ms = 100
```

4. 程序启动时读取配置，热键改变时取消旧注册、注册新热键
5. 增加 `--config <path>` 命令行参数支持临时配置

### 验收标准

- [ ] 修改 `config.toml` 中的热键（如改为 `Alt+Up`），重启后新热键生效
- [ ] 修改滚动速度参数，重启后生效
- [ ] 配置文件不存在时，程序自动生成默认配置文件（含注释说明）
- [ ] 配置文件格式错误时，程序回退到默认配置并打印日志（不崩溃）
- [ ] `keyscroll --config myconfig.toml` 可指定配置文件

### 工作量

约 150-200 行代码（含 TOML 序列化、文件 I/O、热键字符串解析），1-2 天。

---

## Phase 4 — 系统托盘图标 + 可视反馈

> 后台程序需要状态可见性。系统托盘是 Windows 后台工具的标配交互入口。

### 具体任务

1. 注册系统托盘图标（`NOTIFYICONDATA`），右键菜单：
   - 📄 编辑配置文件
   - 🔄 重新加载配置
   - 🟢 启用 / ⏸ 暂停（Ctrl+↑/↓ 临时失效）
   - ❌ 退出
2. 左键点击显示状态弹窗（toast）：当前快捷键绑定、启/停状态
3. 滚动时在鼠标指针附近显示微小的浮动指示器（可选，轻量实现）：
   - 一个半透明箭头 ↑ 或 ↓ 跟随鼠标位置，持续 300ms 后渐隐
   - 可用分层窗口（`WS_EX_LAYERED` + `UpdateLayeredWindow`）实现
4. 最小化时隐藏在托盘，不占任务栏

### 验收标准

- [ ] 系统托盘可见程序图标，右键菜单四个选项均有效
- [ ] 点击"暂停"后，热键不再触发射击；点击"启用"后恢复
- [ ] 点击"重新加载配置"后立即读取最新配置并更新热键，无需重启
- [ ] 浮动指示器（如果有）清晰可见但不过分突兀
- [ ] 程序关闭后托盘图标消失，无残留

### 工作量

约 250-350 行代码，2-3 天。浮动指示器 ≈ 附加 1 天。

---

## Phase 5 — 开机自启 + 安装体验

> 让工具更像一个正式产品，用户装好就不用管了。

### 具体任务

1. 开机自启（写注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`），提供 `--install` / `--uninstall` 参数管理
2. 打包为 MSI 安装包（WiX Toolset），包含：
   - 安装到 `Program Files\KeyScroll`
   - 注册开机自启
   - 可选——添加快捷方式到开始菜单
3. 单文件模式：也支持仅复制 EXE 到任意目录手动运行（绿色模式）
4. 日志系统：滚动到 `%APPDATA%/keyscroll/logs/`，按日期轮转，保留 7 天

### 验收标准

- [ ] `keyscroll --install` 写入开机自启，重启后进程自动运行
- [ ] `keyscroll --uninstall` 移除开机自启项
- [ ] MSI 安装包正常安装、卸载（控制面板"添加/删除程序"可见）
- [ ] 绿色模式（复制 EXE 直接运行）仍然完全可用
- [ ] 日志按日期存档，超过 7 天的自动清理

### 工作量

安装包制作约 0.5 天，其余 0.5 天，合计 1 天。

---

## Phase 6 — 高级特性与打磨

> 非核心但有用锦上添花的功能。

### 具体任务

1. **⚡ 瞬时滚屏模式**：`Ctrl+Shift+Space` 触发一次大跳跃（比如 6 行等价量），适合快速翻页，不持续
2. **🪟 每应用配置**：对不同进程名应用不同的热键 / 速度
   - 配置文件支持 `[app."chrome.exe"]` 独立覆盖
   - 检测前台窗口（`GetForegroundWindow` + `GetWindowModuleFileName`），按进程名选择配置
3. **🔄 滚动方向指示**：系统托盘图标根据当前滚动方向旋转 / 变色
4. **🌍 多显示器**：多 DPI 环境下 `SendInput` 坐标处理无误
5. **🧪 自动化测试**：添加集成测试，用 `SendMessage` 发送 `WM_HOTKEY` 模拟热键触发，验证滚动事件是否发出
6. **🔒 防误触保护**：可选——仅当鼠标在可滚动区域内才响应（窗口类排除 `Static`、`Button` 等）

### 验收标准

- [ ] 瞬时滚屏模式按一次跳 6 行（可配置），不持续
- [ ] Chrome 中与 VSCode 中使用不同的滚动速度 / 热键组合
- [ ] 多显示器 + 不同缩放比率下滚动无异常
- [ ] 集成测试覆盖核心热键注册 → 接收 → 事件发送链路
- [ ] 防误触模式开启后，在不可滚动控件上不会误触发

### 工作量

每项约 0.5-1 天，总计 3-5 天。

---

## 汇总时间线

| Phase | 内容 | 估算时间 | 交付物 |
|-------|------|----------|--------|
| Phase 0 | AHK 快速原型 | **1 小时** | `keyscroll.exe`（AHK 编译） |
| Phase 1 | Rust 核心 + 热键注册 | **1-3 天** | `keyscroll.exe`（Rust 原生） |
| Phase 2 | 滚动行为精细化 | **1-2 天** | 加速度、平滑停止 |
| Phase 3 | 配置文件 | **1-2 天** | `config.toml` 驱动 |  
| Phase 4 | 系统托盘 + 视觉反馈 | **2-3 天** | 托盘菜单、浮动指示 |
| Phase 5 | 安装部署 | **1 天** | MSI 安装包、自启 |
| Phase 6 | 高级特性 | **3-5 天** | 每应用配置、测试 |
| **总计** | | **~10-17 天** | |

---

## 推荐执行策略

### 🥇 对于"今天就要用"
- 只做 Phase 0，30 分钟出活儿
- 如果 AHK 编译后的 exe 足够稳定，停在 Phase 0 也完全 OK

### 🥈 对于"做个正经项目"
- Phase 0 → Phase 1 → Phase 3 → Phase 4 → Phase 5 → Phase 2 → Phase 6
- 即**先把配置系统做出来再精细化行为**，因为 Phase 2 的加速度参数需要配置驱动才能调优

### 🥉 对于"想全面可控"
- Phase 0 → 按编号顺序推进，每一步都有明确可交付物

---

## 技术选型说明

| 方案 | 优点 | 缺点 | 推荐场景 |
|------|------|------|----------|
| **AutoHotkey v2** | 极快实现，~20 行代码，可编译为单 exe | Windows only，exe ~1MB，非 Rust | Phase 0 快速验证 |
| **Rust + windows-rs** | 单二进制 < 1MB，无运行时，原生性能，跨 Windows 版本 | 需要 Rust 编译器，Windows API 学习曲线 | 正式项目首选 |
| **Python + pynput** | 跨平台，快速开发 | 需 Python 运行时或 PyInstaller 打包（~30MB），后台常驻内存高 | 不推荐 |
| **Tauri** | 跨平台，前端 UI 丰富 | 严重 overkill，内存占用过高 | 不推荐 |

**结论**：Phase 0 AHK 即战力；Phase 1+ 选择 Rust。

---

## 风险与应对

| 风险 | 概率 | 应对 |
|------|------|------|
| 热键冲突（Ctrl+↑ 已被 Excel/IDE 占用） | 高 | Phase 3 配置可更换；默认键位可选 `Alt+↑/↓` 作为备选 |
| 部分应用拦截 `SendInput` 模拟事件 | 中 | 备用路线：Phase 6 改用 `mouse_event` 或驱动级注入 |
| 游戏反作弊系统误判 | 低 | 文档说明非游戏工具；不影响正常游戏 |  |
| Windows UAC 或安全软件误报 | 低 | 代码签名证书；开源可审查；申请微软 Defender 白名单 |

---

*本计划由 Sisyphus 于 2026-06-29 生成*