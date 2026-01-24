#!/usr/bin/env python3
"""
フォントサブセット生成スクリプト
Rustソースコードから使用文字を抽出し、必要な文字のみ含むフォントを生成する
"""
import re
import subprocess
from pathlib import Path

# プロジェクトルート
PROJECT_ROOT = Path(__file__).parent.parent
RUST_SRC_DIR = PROJECT_ROOT / "cross-section-ui" / "src"
DATA_DIR = PROJECT_ROOT / "data"
FONT_SRC = PROJECT_ROOT / "cross-section-ui" / "static" / "NotoSansJP-Regular.ttf"
FONT_DST = PROJECT_ROOT / "cross-section-ui" / "static" / "NotoSansJP-Subset.ttf"

# 基本文字セット（常に含める）
BASE_CHARS = (
    # ASCII
    " !\"#$%&'()*+,-./0123456789:;<=>?@"
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`"
    "abcdefghijklmnopqrstuvwxyz{|}~"
    # 記号
    "°±×÷"
    "←→↑↓"
    "△▽○●◎◐◑◒◓"
    # 単位
    "㎜㎝ｍ㎞²³"
)

def extract_japanese_chars(text: str) -> set:
    """テキストから日本語文字を抽出"""
    chars = set()
    # 日本語文字（ひらがな、カタカナ、漢字）
    japanese = re.findall(r'[ぁ-んァ-ヶー一-龯々〆〤]', text)
    chars.update(japanese)
    return chars

def extract_chars_from_rust(src_dir: Path) -> set:
    """Rustソースディレクトリ内の全.rsファイルから文字を抽出"""
    chars = set()
    for filepath in src_dir.glob("**/*.rs"):
        try:
            text = filepath.read_text(encoding="utf-8")
            extracted = extract_japanese_chars(text)
            chars.update(extracted)
            if extracted:
                print(f"  {filepath.name}: {len(extracted)} 文字")
        except Exception as e:
            print(f"  {filepath.name}: 読み込みエラー ({e})")
    return chars

def extract_chars_from_data(data_dir: Path) -> set:
    """データファイル（JSON, CSV）から文字を抽出"""
    chars = set()
    for pattern in ["*.json", "*.csv"]:
        for filepath in data_dir.glob(pattern):
            try:
                text = filepath.read_text(encoding="utf-8")
                extracted = extract_japanese_chars(text)
                chars.update(extracted)
                if extracted:
                    print(f"  {filepath.name}: {len(extracted)} 文字")
            except Exception as e:
                print(f"  {filepath.name}: 読み込みエラー ({e})")
    return chars

def main():
    print("=== フォントサブセット生成 ===")

    # 使用文字を収集
    chars = set(BASE_CHARS)

    # Rustソースから抽出
    if RUST_SRC_DIR.exists():
        print("Rustソースから抽出:")
        rust_chars = extract_chars_from_rust(RUST_SRC_DIR)
        chars.update(rust_chars)
        print(f"Rustソースから合計 {len(rust_chars)} 文字抽出")

    # データファイルから抽出
    if DATA_DIR.exists():
        print("データファイルから抽出:")
        data_chars = extract_chars_from_data(DATA_DIR)
        chars.update(data_chars)
        print(f"データファイルから {len(data_chars)} 文字抽出")

    # ユニコードポイントのリストを作成
    unicodes = ",".join(f"U+{ord(c):04X}" for c in sorted(chars))

    print(f"合計: {len(chars)} 文字")
    print(f"入力: {FONT_SRC} ({FONT_SRC.stat().st_size / 1024 / 1024:.1f} MB)")

    # pyftsubsetでサブセット生成
    cmd = [
        "pyftsubset",
        str(FONT_SRC),
        f"--output-file={FONT_DST}",
        f"--unicodes={unicodes}",
        # "--flavor=woff2",  # WOFF2形式で圧縮（include_bytes!用にTTFのまま）
        "--layout-features=*",  # レイアウト機能を保持
    ]

    try:
        subprocess.run(cmd, check=True)
        print(f"出力: {FONT_DST} ({FONT_DST.stat().st_size / 1024:.1f} KB)")
        print("完了!")

        # 削減率を表示
        original = FONT_SRC.stat().st_size
        subset = FONT_DST.stat().st_size
        reduction = (1 - subset / original) * 100
        print(f"削減率: {reduction:.1f}%")

    except subprocess.CalledProcessError as e:
        print(f"エラー: {e}")
        return 1
    except FileNotFoundError:
        print("エラー: pyftsubsetが見つかりません")
        print("pip install fonttools[woff] でインストールしてください")
        return 1

    return 0

if __name__ == "__main__":
    exit(main())
