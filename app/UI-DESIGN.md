# Ferry摇摆人 UI 设计方向：工具台（Ink & Vermilion）

产品版本 0.1.0，应用名 `ferry`，窗口标题和界面品牌使用 **Ferry摇摆人**。

界面不是通用 SaaS 仪表盘，而是一台桌面工具：深墨色导航 rail、纸白工作区、
细线分隔、衬线标题、一个与品牌图标一致的青蓝信号色。所有页面复用同一套组件。

设计规则来源于
[unslop-ui-skill](https://github.com/claudiusararu/unslop-ui-skill)（MIT），
已安装在 `.opencode/skills/no-ai-slop/`。

## 色板

| Token | 值 | 用途 |
|---|---|---|
| `INK` | `#171A1C` | 主文字、深色 rail 附近的强调 |
| `INK_SOFT` | `#3B4145` | 次级正文（白底对比度 > 10:1） |
| `MUTED` | `#5F666A` | 说明文字、元数据（白底约 5.8:1） |
| `FAINT` | `#8A9092` | 禁用/占位，不用于正文 |
| `PAPER` | `#FFFFFF` | 卡片与状态栏表面 |
| `CANVAS` | `#F2F2EE` | 工作区底色 |
| `HAIRLINE` | `#DDDED8` | 卡片和分隔线 |
| `RAIL` | `#191C1E` | 左侧导航 |
| `ACCENT` | `#0A6E96` | 主按钮、选中态、rail 活动标记（取自品牌图标） |
| `ACCENT_DARK` | `#085E80` | 主按钮 hover、浅色底上的青色文字 |
| `ACCENT_SOFT` | `#E3F0F5` | 选中标签、hover 浅底 |
| `SUCCESS` | `#2E6B4F` | 完成 |
| `WARN` | `#856014` | 跳过/警告 |
| `ERROR` | `#9C2A1F` | 失败 |

一个强调色，不铺满。状态色只用于状态标签和状态点；红色只保留给危险操作和错误。

## 字体

- 正文：Source Han Sans SC（`SourceHanSansSC-Regular.otf`），回退 Noto Sans
  CJK、文泉驿微米黑、DejaVu Sans。
- 标题：Source Han Serif SC Medium（`SourceHanSerifSC-Medium.otf`），
  回退 Noto Serif CJK。页面标题 30px，卡片标题 16-19px。
- 等宽：Source Han Mono / DejaVu Sans Mono，仅用于路径、版本、数字。
- 字体在 `src/main.rs::install_fonts` 里加载；`display` 是一个自定义
  `FontFamily`，在 `src/ui.rs` 里通过 `display(size)` 使用。

## 组件

- **卡片**：`PAPER` 填充、1px `HAIRLINE`、6px 圆角、16px 内边距、无阴影。
  同一列里的卡片强制同宽（`card()` 内部 `set_width`）。
- **按钮**：5px 圆角、34px 高。Primary 青蓝底白字；Secondary 白底细边；
  Ghost 透明底；DangerGhost 红色文字。hover / pressed / disabled 都由
  `button_w()` 统一绘制，不依赖 egui 默认按钮样式。
- **Chip**：4px 圆角、30px 高。选中青蓝，未选中纸白细边，hover 浅青蓝底。
- **标签**：3px 圆角、21px 高，浅色底 + 同色系深色文字。
- **分隔线**：1px `HAIRLINE` 的 `rule()`，不用阴影做层级。
- **图标**：`paint_nav_icon()` 用 painter 画的 6 个单色线性图标（视频/音频/
  图片/合并/队列/设置），1.5px 描边。禁止 emoji 当图标。

## 布局

- 左侧 200px 深色 rail：32px 品牌图标 + “Ferry摇摆人”、编号导航（01-06）、
  底部版本、ffmpeg 状态和队列提示。品牌区不再放副标题。品牌图标来自 `app/deb/icon.png`，运行时
  通过 `include_bytes!` 嵌入，同时作为窗口图标。
- 主区 26px 内边距；页面标题 + 说明 + 细线 + 内容。
- 转换页：默认宽度下主区 500px + 288px 任务摘要两栏；内容宽度 < 720px 时堆叠。
- 图片转换页的任务摘要里有“输出尺寸”宽高输入：0 表示原始尺寸，只填一边时
  按比例缩放；ICO 固定为单帧 256px，不显示宽高输入。尺寸在添加任务时写入
  `Job.resize`，不影响已排队的任务。
- 合并与混流：宽度 ≥ 760px 时 56/44 两栏，否则堆叠。
- 设置：输出卡片通栏；音质和关于在 ≥ 700px 时两栏。
- 所有页面包在 `ScrollArea` 里，窄窗口下可滚动，不裁切关键操作。
- 转换页和工具页的空状态都给出下一步动作；队列空状态给出“去添加文件”。

## 文案

- 用具体的动作和数字：`转换 2 个文件为 MP4`，不写“开始转换”。
- 说明转换由谁完成、原文件是否保留、输出在哪里。
- 不用 em-dash、不用“赋能/一键/极致/无缝”这类填充词。
- 中文文案用中文标点；路径和版本用等宽字体。

## 自检

改完 UI 后逐页截图，检查：

- 默认 1060×660 和最小 880×560 都不横向溢出。
- 没有 emoji 图标、紫色/蓝色渐变、低对比灰色正文、统一软阴影；
- 卡片同宽、按钮状态一致、主按钮只有一个明确动作；
- 空状态能教会下一步；没有点了只重绘的假按钮。

截图命令见项目根目录 `AGENTS.md`。
