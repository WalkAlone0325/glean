# 安装

## 系统要求

- **macOS 13.0+**（Ventura 及以上）
- **Apple Silicon (M1/M2/M3/M4) 或 Intel Mac**
- 约 200MB 磁盘空间（应用本体）+ 索引数据空间

## 下载

前往 [Releases](https://github.com/WalkAlone0325/glean/releases/latest) 下载对应架构的 `.dmg`：

| 文件名 | 适用 |
|--------|------|
| `Glean_<version>_aarch64.dmg` | Apple Silicon (M 系列) |
| `Glean_<version>_x64.dmg` | Intel Mac |

## 安装步骤

1. 双击下载的 `.dmg` 文件
2. 在弹出的窗口中，将 **Glean** 图标拖到 **Applications** 文件夹
3. 打开 **Applications** 文件夹，双击 Glean 启动

## 解除 macOS 隔离（首次启动）

由于应用当前未签名，macOS 会阻止启动并提示"无法验证开发者"。

**方式一**：在终端执行（推荐）

```bash
xattr -dr com.apple.quarantine /Applications/Glean.app
```

**方式二**：右键 → 打开 → 在弹窗中点击"打开"

**方式三**：系统设置 → 隐私与安全性 → 滚动到底部 → 点击"仍要打开"

## 从源码构建

如果你希望自行编译：

```bash
# 1. 克隆仓库
git clone https://github.com/WalkAlone0325/glean.git
cd glean

# 2. 安装依赖
pnpm install

# 3. 开发模式（热重载）
pnpm tauri dev

# 4. 生产构建
pnpm tauri build

# 产物位于
# src-tauri/target/release/bundle/dmg/Glean_<version>_aarch64.dmg
```

### 前置依赖

- Node.js 20+
- pnpm 11+
- Rust 1.75+（`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`）
- Xcode Command Line Tools（`xcode-select --install`）

## 卸载

删除应用本体和用户数据：

```bash
# 删除应用
rm -rf /Applications/Glean.app

# 删除所有用户数据（索引、对话、配置）
rm -rf ~/Library/Application\ Support/com.glean.app/
```
