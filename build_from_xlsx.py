#!/usr/bin/env python3
"""
Excel→JSON→Wasmビルドの一括スクリプト

使い方:
  python build_from_xlsx.py                  # デフォルトの計画.xlsxを使用
  python build_from_xlsx.py path/to/file.xlsx  # 指定したExcelを使用
  python build_from_xlsx.py --watch          # ファイル変更を監視して自動ビルド
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

# プロジェクトのルートディレクトリ
PROJECT_ROOT = Path(__file__).parent


def parse_xlsx_to_json(xlsx_path: Path, output_path: Path) -> bool:
    """ExcelファイルをパースしてJSONに変換"""
    print(f"[1/3] Parsing: {xlsx_path}")

    # xlsx_parser.pyを実行
    result = subprocess.run(
        [
            sys.executable,
            str(PROJECT_ROOT / "src" / "xlsx_parser.py"),
            str(xlsx_path),
            str(output_path),
        ],
        capture_output=True,
        text=True,
        encoding="utf-8",
    )

    if result.returncode != 0:
        print(f"Error: {result.stderr}")
        return False

    print(result.stderr)  # パーサーの出力はstderrに書かれる

    if output_path.exists():
        print(f"[2/3] Wrote: {output_path}")
        return True
    print(f"Error: {output_path} not found")
    return False


def build_wasm(release: bool = True) -> bool:
    """Wasmをビルド"""
    print("[3/3] Building Wasm...")
    ui_dir = PROJECT_ROOT / "cross-section-ui"

    cmd = ["trunk", "build"]
    if release:
        cmd.append("--release")

    result = subprocess.run(cmd, cwd=ui_dir)
    return result.returncode == 0


def main():
    parser = argparse.ArgumentParser(description="Excel→JSON→Wasmビルド")
    parser.add_argument("xlsx", nargs="?", help="Excelファイルパス")
    parser.add_argument("--watch", action="store_true", help="ファイル監視モード")
    parser.add_argument("--no-build", action="store_true", help="Wasmビルドをスキップ")
    args = parser.parse_args()

    # デフォルトのExcelパス
    if args.xlsx:
        xlsx_path = Path(args.xlsx)
    else:
        xlsx_path = PROJECT_ROOT / "data" / "計画.xlsx"

    if not xlsx_path.exists():
        print(f"Error: {xlsx_path} not found")
        sys.exit(1)

    # 出力先
    output_path = PROJECT_ROOT / "data" / "sections.json"

    if args.watch:
        print(f"Watching: {xlsx_path}")
        print("Press Ctrl+C to stop")
        last_mtime = xlsx_path.stat().st_mtime

        while True:
            try:
                time.sleep(1)
                mtime = xlsx_path.stat().st_mtime
                if mtime > last_mtime:
                    print(f"\n[{time.strftime('%H:%M:%S')}] File changed, rebuilding...")
                    last_mtime = mtime
                    if parse_xlsx_to_json(xlsx_path, output_path):
                        if not args.no_build:
                            build_wasm()
                    print("Done. Watching for changes...")
            except KeyboardInterrupt:
                print("\nStopped.")
                break
    else:
        # 一回だけ実行
        if not parse_xlsx_to_json(xlsx_path, output_path):
            sys.exit(1)

        if not args.no_build:
            if not build_wasm():
                sys.exit(1)

        print("\nSuccess!")


if __name__ == "__main__":
    main()
