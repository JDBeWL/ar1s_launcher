#!/usr/bin/env python3
"""生成界面用的中文子集字体（public/fonts/noto-sans-sc-subset.woff2）。

背景
----
桌面端不能依赖系统字体：Windows 是微软雅黑、macOS 是苹方、Linux 是 Noto Sans CJK，
三者的字面尺寸和墨迹位置都不同，同一份 CSS 在不同机器上会差 1px 级别。
所以中文也随包自带一份。

做法
----
拿 Noto Sans SC 的**可变字体**（wght 100-900，17.8 MB）裁出实际要用的字，
输出仍然是可变字体 —— 一个文件覆盖全部字重，比按字重各存一份静态字体省一半体积。

字符集 = 以下四部分的并集：
  1. 界面实际用字：扫描 src/ 与 src-tauri/src/ 的源码，取出所有 CJK/标点/全角字符；
     Rust 那边的报错文案也会显示到界面上，所以要一起扫。
  2. GB2312 全表：一级（3755）+ 二级（3008）汉字 + 符号区。动态文本用得到 ——
     实例名、整合包标题、启动器转述的游戏日志，这些不可能靠扫源码穷举。
     6763 字覆盖现代汉语约 99.7%，符号区补上希腊/俄文字母、圈码、箭头、制表符。
  3. 拉丁基本区 + Latin-1：给字体留一份自足能力，Roboto 加载失败时也能兜住 ASCII。
  4. 界面里出现的排版符号：破折号、省略号、书名号、全角标点等。

用法
----
    pip install "fonttools[woff]" brotli
    python scripts/build-cjk-subset.py

依赖只在这条维护命令里用到，不参与 pnpm build / tauri build。

产物
----
    public/fonts/noto-sans-sc-subset.woff2   (约 2.0 MB)
    public/fonts/NotoSansSC-OFL.txt          (OFL-1.1 授权文本，必须随包分发)

改完界面文案后重新跑一次即可；漏掉的生僻字不会变成豆腐块，
浏览器会逐字回退到系统 CJK 字体，只是那些字不受版本一致性保护。
"""

from __future__ import annotations

import pathlib
import sys
import urllib.request

# Noto Sans SC 可变字体（Google Fonts 官方仓库，OFL-1.1）
SOURCE_URL = "https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf"
LICENSE_URL = "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/LICENSE"

ROOT = pathlib.Path(__file__).resolve().parent.parent
CACHE = ROOT / ".cache" / "NotoSansSC-VF.ttf"
OUT_FONT = ROOT / "public" / "fonts" / "noto-sans-sc-subset.woff2"
OUT_LICENSE = ROOT / "public" / "fonts" / "NotoSansSC-OFL.txt"

# 扫描这些目录下的源码，收集界面用字
SCAN_DIRS = ("src", "src-tauri/src")
SCAN_EXT = {".vue", ".ts", ".css", ".rs", ".html"}

# 额外强制保留的排版符号（界面里用到的）
EXTRA_SYMBOLS = "—‘’“”…•·×→✓−"


def collect_charset() -> set[str]:
    chars: set[str] = set()

    for d in SCAN_DIRS:
        for p in (ROOT / d).rglob("*"):
            if p.is_file() and p.suffix in SCAN_EXT:
                for ch in p.read_text(encoding="utf-8", errors="ignore"):
                    o = ord(ch)
                    if o > 0x2000:  # 汉字、标点、全角字符都在这个区间之上
                        chars.add(ch)

    # GB2312 全表：0xA1-0xA9 是符号区（拉丁/希腊/俄文字母、圈码、箭头、制表符，
    # 整合包描述和文件路径里会出现），0xB0-0xF7 是一级(3755)+二级(3008)汉字
    for hi in range(0xA1, 0xF8):
        for lo in range(0xA1, 0xFF):
            try:
                chars.add(bytes([hi, lo]).decode("gb2312"))
            except UnicodeDecodeError:
                pass

    chars |= {chr(c) for c in range(0x20, 0x7F)}  # ASCII 可打印区
    chars |= {chr(c) for c in range(0xA0, 0x100)}  # Latin-1
    chars |= set(EXTRA_SYMBOLS)
    return chars


def download(url: str, dest: pathlib.Path) -> pathlib.Path:
    if dest.exists():
        print(f"复用缓存 {dest.relative_to(ROOT)}")
        return dest
    dest.parent.mkdir(parents=True, exist_ok=True)
    print(f"下载 {url}")
    with urllib.request.urlopen(url) as r, dest.open("wb") as f:  # noqa: S310
        f.write(r.read())
    print(f"  -> {dest.relative_to(ROOT)} ({dest.stat().st_size / 1e6:.1f} MB)")
    return dest


def main() -> int:
    try:
        from fontTools import subset
        from fontTools.ttLib import TTFont
    except ImportError:
        print('缺少依赖，先执行： pip install "fonttools[woff]" brotli', file=sys.stderr)
        return 1

    source = download(SOURCE_URL, CACHE)

    chars = collect_charset()
    print(f"字符集：{len(chars)} 个字符")

    OUT_FONT.parent.mkdir(parents=True, exist_ok=True)
    options = subset.Options()
    options.flavor = "woff2"
    options.layout_features = ["*"]  # 保留 kern/locl/vert 等全部特性，CJK 标点位置依赖它们
    options.name_IDs = ["*"]
    options.drop_tables += ["DSIG"]

    font = TTFont(str(source))
    subsetter = subset.Subsetter(options=options)
    subsetter.populate(text="".join(sorted(chars)))
    subsetter.subset(font)
    font.flavor = "woff2"
    font.save(str(OUT_FONT))

    # 校验：覆盖是否完整、可变轴是否还在
    check = TTFont(str(OUT_FONT))
    cmap = check.getBestCmap()
    missing = [c for c in chars if ord(c) not in cmap]
    print(f"产出：{OUT_FONT.relative_to(ROOT)} ({OUT_FONT.stat().st_size / 1e6:.2f} MB)")
    print(f"  glyphs={check['maxp'].numGlyphs}  可变轴={'fvar' in check}  缺失字符={len(missing)}")
    if missing:
        print("  警告：以下字符未进子集 ->", "".join(missing)[:50], file=sys.stderr)

    download(LICENSE_URL, OUT_LICENSE)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
