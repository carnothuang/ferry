# 机制与参数表 — Ferry摇摆人 0.1.0（格式工厂 5.22.2.0 的简洁重建版）

## 0. 安装包本体（静态分析结果）

**已核实（observed）**：`FormatFactory_setup.exe` 是“格式工厂”5.22.2.0（Geshi Inc.）的
Windows NSIS 安装程序，164 MB，带数字签名证书。外层资源仅含安装器对话框、图标与版本信息
（ProductVersion 5.22.2.0）。

**重要限制**：安装器载荷（它真正安装的应用文件）是压缩/加密的 NSIS 数据块。7z 与内置解析器
都无法列出内部文件表，载荷内也没有明文痕迹（无 MZ 头、无 ffmpeg/libmp3lame 等标记字符串）。
干净房间规则禁止运行安装程序，因此**无法从原始安装包静态提取原版应用的参数表**。
下文所有参数均标注来源：`estimated`（按同类工具的通行做法估计）或引用官方 ffmpeg 默认值
（ffmpeg 4.x–7.x 的一致行为，属公开工具行为，非逆向产物）。

## 1. 功能范围（重建版实现清单）

格式工厂这类工具的核心功能，按"功能相同、界面更简洁"的要求实现：

| 模块 | 功能 | 状态 |
|---|---|---|
| 视频 | MP4 / MKV / WebM / GIF / WMV / AVI / MOV，H.264 编码 | ffmpeg，H.264 + AAC 默认 |
| 音频 | MP3 / WAV / AAC / FLAC / OGG / M4A，可选音质等级 | 同上 |
| 图片 | JPG / PNG / WebP / BMP / GIF / TIFF / ICO / TGA，可设置输出宽高 | 独立实现，不依赖 ffmpeg |
| 附加 | 合并（concat 同类文件）、混流（视频+音轨封装） | concat demuxer / -c copy |
| 转换 | 批量转换队列、每项状态、转换时保留原文件 | ffmpeg 子进程 |
| 设置 | 输出目录、改名方式、同名处理、音质、是否覆盖 | 本地 JSON 配置 |

## 2. 编码参数表（全部 estimated / ffmpeg 公开默认值）

### 视频（视频编码均 estimated：格式工厂公开版以 H.264 为默认编码）
| 参数 | 值 | 来源 |
|---|---|---|
| 视频编码 | libx264（CRF 23，preset medium） | estimated（ffmpeg 公开默认） |
| 音频编码 | AAC 128 kbps，48 kHz | estimated |
| 目标 FPS | 保持原速（不强制） | estimated |
| 分辨率 | 保持原分辨率 | estimated |
| MP4 默认编码器串 | `-c:v libx264 -crf 23 -preset medium -c:a aac -b:a 128k -movflags +faststart` | estimated |

### 音频（音质等级：值 estimated，等级结构是同类工具通行做法）
| 等级 | MP3 比特率 |
|---|---|
| 高音质 | 320 kbps |
| 标准 | 192 kbps |
| 普通音质 | 128 kbps |

### 图片（独立解码，不依赖 ffmpeg；值 estimated / 格式规定）
| 目标格式 | 固定参数 |
|---|---|
| JPG | RGB8，固定质量 90（quality=90，格式规定的硬性范围 1–100） |
| WebP | 固定质量 90（1–100） |
| PNG / BMP / TGA | 无损，无质量参数 |
| GIF / TIFF / ICO | 由 image crate 默认编码器处理 |
| 输出尺寸 | 可设置宽、高；0 表示原始尺寸；只填一边时按比例缩放；单边最大 12000 px（重建版约定，estimated） |
| 降采样 | 未指定输出尺寸时，PNG/JPG/WebP/BMP/TGA/GIF 最大边 4000 px（重建版约定，estimated） |
| ICO | 单尺寸 256 px 单帧（重建版约定，estimated），忽略自定义宽高 |

## 3. 输出命名规则（重建版约定，estimated）
- 默认规则：`原文件名_格式后缀.扩展名`（例如 `clip.mp4` → `clip_mp4.mp4`）。
- 同名冲突：默认覆盖（overwrite），可选改名（自动加序号）或跳过。
- 输出目录默认与源文件同目录。

## 4. 行为约定（estimated）
- 转换在后台线程逐个执行（简单可靠，比并行转换更省资源）。
- 失败项标红，可重试；转换不删除原文件。
- 图片转换用内置解码器；视频/音频/合并/混流需要系统装有 ffmpeg。
- GIF 目标（视频→GIF）：15 fps、480px 宽、固定 16 色调色板、最高 30 秒，单线程解码。

## 5. 反编译证据（Deep 模式 / Ghidra，2026-11-09 补充）
对外壳 PE 全量反编译（89 个函数全部清点，关键函数逐个生成伪代码）。结论：**本文件是重建/混淆过的 NSIS 2.46.5-ANSI 安装器外壳，不含任何媒体转换代码**。原版的转换程序与编码参数在载荷数据块内，静态不可恢复（证据见下）。

| 函数地址 | 作用（反编译所得） | 证据 |
|---|---|---|
| `entry` @ `0x403391` | NSIS 引导：解析 `/S` `/NCRC` `/D=path`，取临时目录，OleInitialize，出错路径只有 NSIS 错误框 | ev_40276d0a… |
| `sub_403030` @ `0x403030` | 读取首部块并校验**自定义魔数**（4 个常量 `0xA7190845`/`0x2617733F`/`0xADE48183`/`0x980D3F65`，非常规 NSIS 的 "NullsoftInst" ASCII 值）→ 解释了 7z 无法解析；随后读入解压头、定位数据块 | ev_077c1bae… |
| `sub_402c25` @ `0x402c25` | 进度对话框："unpacking data: %d%%" / "verifying installer: %d%%"，250ms 定时器 | ev_a3c16bcc… |
| `sub_401599` @ `0x401599` | NSIS 脚本虚拟机分发器（1103 行伪代码）：操作码 4…0x26+ 全是文件复制、注册表、INI、消息框等安装操作，无媒体操作 | ev_b4f6e1de… |
| `sub_405f7e` @ `0x405f7e` | Shell 特殊文件夹解析（CSIDL → 桌面/开始菜单路径），安装器运行时逻辑 | ev_45b5edd3… |
| `sub_40699f` @ `0x40699f` | **LZMA 范围解码器**（公开 LZMA SDK：概率表初值 0x400、11 位区间、0x1000000 归一化、状态机 0…0x1C），即载荷的解压器 | ev_2275b996… |
| `sub_401000` @ `0x401000` | 安装向导窗口过程：渐变背景绘制（逐 4px 条带线性插值）+ 品牌文字（DrawTextA） | ev_fe54d66b… |
| `sub_404380` @ `0x404380` | 组件选择页：树形控件、勾选位运算（0x01/0x10/0x40/0x80）、ImageList 图标 | ev_0ecea9db… |
| `sub_404e38` @ `0x404e38` | 向导页切换引擎：上一步/下一步/取消按钮（IDC 1/2/3）、对话框页创建与导航 | ev_78b5fc57… |
| `sub_4052e6` @ `0x4052e6` | 安装器主流程启动：读 ResourceLocale 注册表、语言 DLL、注册 "_Nb" 主窗口、DialogBoxParamA 进入向导 | ev_35e4d930… |
| `sub_405b3d` @ `0x405b3d` | "程序是否已在运行"检测：PSAPI EnumProcesses / Toolhelp32 快照枚举进程比对文件名 | ev_0b5c2de8… |
| `sub_406566` @ `0x406566` | 语言 ini 读取（"name" 节、codepage 0x4e4=1252、WideChar 转换）——安装器语言支持 | ev_601bfa79… |
| `sub_40639b` @ `0x40639b` | 递归删除目录树（FindFirstFileA/DeleteFileA/RemoveDirectoryA）——安装器清理逻辑 | ev_c36568e0… |
| `sub_4061d6` @ `0x4061d6` | 写 wininit.ini 的 [Rename] 段（重启后替换/删除占用文件）——安装器重启替换逻辑 | ev_d810b96a… |
| `sub_405716` @ `0x405716` | 大小写不敏感子串查找（lstrcmpiA 循环）——通用字符串助手 | ev_dfb61a2d… |
| `sub_404046`→`FUN_00403f1a` | 安装目录选择页：SHBrowseForFolderA 浏览、GetDiskFreeSpaceA 磁盘空间校验 | ev_a4173171… |

熵图（载荷区，64 KB 块）：偏移 131,928 起所有块熵值 7.995–7.998（上限 8.0）。**该数值不能区分"压缩"与"加密"**（两者均接近上限）；结合无任何容器签名 + 自定义魔数，判定为"压缩或加密、静态不可恢复"。修正此前"已加密"的表述。

可核实的"真实值"（有地址/出处）：产品名与版本 5.22.2.0（版本资源，.rsrc）、NSIS v2.46.5-ANSI（清单，`0x4E5EF8`）、requireAdministrator（清单）、载荷范围 `0x358`–`0x9C9FF6A`（164,056,408 B）。**转换参数（码率/CRF/帧率等）不在本文件内，无从反编译**——重建版沿用机制表标注 estimated 的 ffmpeg 公开默认值，此为诚实的极限。

## 6. 重建版验证结果（在本机实测）
| 项 | 结果 |
|---|---|
| 视频 MP4→MKV（H.264+AAC） | ✓ 通过（selftest，输出可被 ffprobe 读取） |
| 音频 MP4→MP3 192k / WAV→FLAC | ✓ 通过 |
| 图片 PNG→JPG/ICO/WebP/BMP/TGA | ✓ 通过（内置解码器） |
| 合并（concat demuxer） | ✓ 通过（拼接时长正确） |
| `--screenshot` 截图模式 | ✓ 通过（1060×660，界面完整渲染：侧栏、格式选择、按钮、状态栏） |
| deb 包 | ✓ 构建成功：ferry_0.1.0_amd64.deb（含图标与桌面入口，依赖 ffmpeg/zenity/xclip） |

## 7. 明确不做 / 原版可能有而未实现
- 安装包捆带的浏览器或其他捆绑组件——重建版不含任何捆绑组件。
- 原版独有功能（如屏幕录像、DVD/ISO 处理）——无法从载荷证实，暂不实现。
- 输出目录改名、文件信息查看等次级功能可后续加。
