# Ferry摇摆人 / ferry - 项目约定

`ferry`（产品名 **Ferry摇摆人**，版本 0.1.0）是一个 Rust + egui
的 Linux 桌面媒体转换工具：视频 / 音频 / 图片互转，支持合并与混流，用
ffmpeg 处理音视频，图片转换使用内置解码器。界面优先，deb 打包。

## 构建与验证

```bash
cd app
cargo build --release
cargo fmt
./build-deb.sh                 # 生成 app/deb/ferry_0.1.0_amd64.deb
```

截图检查（每页都可单独截图，`--size` 用于窄窗口）：

```bash
cd app
./target/release/ferry --page video    --screenshot /tmp/video.png
./target/release/ferry --page audio    --screenshot /tmp/audio.png
./target/release/ferry --page image    --screenshot /tmp/image.png
./target/release/ferry --page tools    --screenshot /tmp/tools.png
./target/release/ferry --page queue    --screenshot /tmp/queue.png
./target/release/ferry --page settings --screenshot /tmp/settings.png
./target/release/ferry --page video --size 880x560 --screenshot /tmp/narrow.png
```

无显示环境用 `xvfb-run -a -s "-screen 0 1200x800x24"` 包一层。

## UI 改动

- 改界面前先加载 `.opencode/skills/no-ai-slop/` 里的 `no-ai-slop` 技能，
  完整清单在 `TELLS.md`。
- 当前设计方向见 `app/UI-DESIGN.md`。保持同一套卡片、按钮、chip、标签、
  字号、圆角和颜色 token，不要在单页里另起一套。
- 品牌图标是 `app/deb/icon.png`，同时用于窗口图标和左上角品牌区，打包时
  安装到 `/usr/share/icons/hicolor/256x256/apps/ferry.png`。主色取自图标的
  青蓝 `#0A6E96`，红色只用于危险操作和错误。
- Linux 任务栏图标：eframe 0.27 的 `with_icon` 只在 Windows/macOS 生效，
  `apply_linux_window_icon` 通过 x11rb 写入 `_NET_WM_ICON`；同时保留
  `.with_app_id("ferry")` 和 .desktop 里的 `StartupWMClass=ferry`。
- 禁止：emoji 当图标、紫色/蓝色渐变主色、低对比灰色正文、统一软阴影卡片、
  整页胶囊化、假按钮（点了只重绘）、无意义的 eyebrow / pill、破折号式文案。
- 空状态要告诉用户下一步；每个可见控件都要有用户能说出的作用。
- 改完必须截图检查默认尺寸和 `880x560` 窄尺寸，两栏布局在窄屏要能堆叠并滚动。

## 图片输出尺寸

图片转换页可以设置输出宽高，0 表示原始尺寸，只填一边时按比例缩放。尺寸在
添加任务时快照进 `Job.resize`，所以转换途中改设置不会影响已排队的任务。
相关代码：`Settings::image_size`、`Job.resize`、`imaging::convert`。
