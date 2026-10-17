#!/usr/bin/env python3
"""生成界面用的中文子集字体（public/fonts/noto-sans-sc-subset.woff2）。

中文必须随包自带：Windows/macOS/Linux 的系统 CJK 字体字面尺寸与墨迹位置都不同，同一份 CSS 在各机器上渲染不一致。
字符集除扫描 src/ 与 src-tauri/src 得到的界面用字外，还要并上 GB2312 全表 —— 实例名、整合包标题、转述的游戏日志无法靠扫源码穷举。
源文件取可变字体，裁剪后仍是可变字体，一个文件覆盖全部字重。
NotoSansSC-OFL.txt 是 OFL-1.1 要求随包分发的授权文本。
用法：pip install "fonttools[woff]" brotli 后执行 python scripts/build-cjk-subset.py；改完界面文案重跑一次，依赖不进 pnpm/tauri build。
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

SCAN_DIRS = ("src", "src-tauri/src")
SCAN_EXT = {".vue", ".ts", ".css", ".rs", ".html"}

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

    # GB2312：0xA1-0xA9 符号区（希腊/俄文字母、圈码、箭头、制表符），0xB0-0xF7 一二级汉字
    for hi in range(0xA1, 0xF8):
        for lo in range(0xA1, 0xFF):
            try:
                chars.add(bytes([hi, lo]).decode("gb2312"))
            except UnicodeDecodeError:
                pass

    chars |= {chr(c) for c in range(0x20, 0x7F)}
    chars |= {chr(c) for c in range(0xA0, 0x100)}
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
