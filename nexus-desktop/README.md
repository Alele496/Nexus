# Nexus 桌面壳（nexus-desktop）

Nexus 的 Electron 桌面外壳，提供 Codex Desktop 级别的窗口体验。Electron 只做「进程管家 + 原生窗口」——渲染层就是 `nexus-web` 的构建产物，由外壳 spawn 的 `nexus agent serve` 子进程以 HTTP 提供。**Rust 引擎零改动**。

## 功能

- **服务器自管**：外壳生成随机 key（`--secret`）、优先 2419 端口（被占则自动找空闲端口）、轮询 HTTP 就绪后加载窗口；退出时 `taskkill /T /F` 杀掉子进程树（Windows）
- **无边框自绘标题栏**：header 即拖拽区（双击最大化），右侧 Windows 风格 最小化/最大化/关闭 按钮（macOS 流量灯在左侧），最大化状态实时同步
- **托盘常驻**：关窗缩托盘；托盘菜单可 显示/隐藏、浏览器打开、退出
- **原生集成**：文件拖入输入框自动插 `@路径`（`webUtils.getPathForFile`）、窗口隐藏时原生通知（审批请求 / 回复完成）、`Ctrl+Alt+N` 全局唤出、https 外链走系统浏览器
- **安全基线**：`contextIsolation` + `sandbox` + 无 `nodeIntegration`；导航锁定在 agent 服务源内，`file://` 直接拒绝
- **窗口状态持久化**：尺寸/位置/最大化状态存 `userData/window-state.json`

## 前置条件

- Node.js 18+
- `nexus/target/debug/nexus.exe` 已构建（`cd nexus && cargo build -p nexus-bin`）
- `nexus-web/dist` 已构建（`cd nexus-web && npm run build`）

## 开发

```bash
npm install

# 直接跑（spawn 真实 server，加载 nexus-web/dist）
npm run dev

# 渲染层 HMR：先起 vite（cd nexus-web && npm run dev），窗口指向 localhost:5173
npm run dev:hud

# 冒烟测试：启动 → 断言标题栏 DOM/桥接面 → 退出，ok:true 才算过
npm run smoke
```

## 打包

```bash
# 解包目录（release/win-unpacked/），快速验证资源打包是否正确
npm run dist:dir

# 完整安装包：NSIS 安装程序 + portable 单文件
npm run dist
```

`electron-builder` 通过 `extraResources` 把 `nexus.exe`（→ `resources/bin/`）和 `nexus-web/dist`（→ `resources/web/`）打进安装包；主进程打包后从 `process.resourcesPath` 解析两者。

## 冒烟测试

`--smoke` 模式（自动化用）：加载真实页面后断言 header 拖拽区、最小化/最大化/关闭按钮、信箱按钮、侧栏、桌面桥接面（`notify`/`getPathForFile`/`openExternal`）全部就位，且渲染进程无错误栈。`--shot=path.png` 可附加截图。

## 目录结构

```
electron/main.ts     窗口 + 托盘 + 生命周期 + IPC + 导航封锁 + 全局快捷键 + --smoke
electron/server.ts   server 管理器（探测/spawn/就绪轮询/key/杀进程）——纯 Node，可单测
electron/preload.ts  contextBridge（窗口控制、openExternal、getPathForFile、notify）
scripts/test-server.ts  集成测试：真实 nexus.exe spawn→就绪→ACP 握手→wrong-key 拒绝→stop
scripts/gen-tray.ts     生成托盘图标（纯 Node 手写 PNG）
```
