"""
出来形管理図DXFにペーパースペースレイアウトを追加するスクリプト

使い方:
    python add_dekigata_layouts.py dekigata.dxf --scale 50 --pages 10
    python add_dekigata_layouts.py dekigata.dxf --scale 50  # ページ数自動検出
"""
import argparse
import ezdxf
import re
import sys
from pathlib import Path

# A3サイズ (mm)
A3_WIDTH = 420
A3_HEIGHT = 297

# ページ間の間隔 (mm)
PAGE_GAP_MM = 10.0


def detect_pages_from_dxf(doc) -> int:
    """DXFからページ数を自動検出（図面番号テキストから）"""
    msp = doc.modelspace()
    max_page = 0

    # "1/10" や "001-5/10" のようなパターンを探す
    pattern = re.compile(r'(\d+)/(\d+)')

    for entity in msp:
        if entity.dxftype() == 'TEXT' or entity.dxftype() == 'MTEXT':
            text = entity.dxf.text if entity.dxftype() == 'TEXT' else entity.text
            match = pattern.search(text)
            if match:
                total = int(match.group(2))
                if total > max_page:
                    max_page = total

    return max_page


def add_layouts(input_path: str, scale: float, pages: int = None, output_path: str = None):
    """DXFにレイアウトを追加"""

    # 出力パス
    if output_path is None:
        p = Path(input_path)
        output_path = str(p.parent / f"{p.stem}_with_layouts{p.suffix}")

    # DXF読み込み
    print(f"読み込み中: {input_path}")
    doc = ezdxf.readfile(input_path)

    # ページ数検出
    if pages is None:
        pages = detect_pages_from_dxf(doc)
        if pages == 0:
            print("エラー: ページ数を自動検出できませんでした。--pages オプションで指定してください。")
            sys.exit(1)
        print(f"検出したページ数: {pages}")

    # DXF単位での計算
    page_height_dxf = A3_HEIGHT * scale
    page_gap_dxf = PAGE_GAP_MM * scale
    frame_width_dxf = A3_WIDTH * scale

    # ビューポート用レイヤー
    if "VIEWPORTS" not in doc.layers:
        vp_layer = doc.layers.add("VIEWPORTS")

    # 既存のLayout1を削除
    if "Layout1" in doc.layouts:
        doc.layouts.delete("Layout1")

    # 各ページのレイアウトを作成
    for page_idx in range(pages):
        layout_name = f"Page_{page_idx + 1}"

        # 既存レイアウトがあればスキップ
        if layout_name in doc.layouts:
            print(f"  スキップ: {layout_name} (既存)")
            continue

        layout = doc.layouts.new(layout_name)

        # A3横向きページ設定
        layout.page_setup(
            size=(A3_WIDTH, A3_HEIGHT),
            margins=(0, 0, 0, 0),
            units="mm"
        )

        # モデル空間でのページ位置
        page_y = page_idx * (page_height_dxf + page_gap_dxf)
        center_x = frame_width_dxf / 2
        center_y = page_y + page_height_dxf / 2

        # ビューポート追加
        vp = layout.add_viewport(
            center=(A3_WIDTH / 2, A3_HEIGHT / 2),
            size=(A3_WIDTH, A3_HEIGHT),
            view_center_point=(center_x, center_y, 0),
            view_height=page_height_dxf,
            dxfattribs={"layer": "VIEWPORTS"}
        )

        print(f"  追加: {layout_name} (center_y={center_y:.0f})")

    # 保存
    doc.saveas(output_path)
    print(f"保存完了: {output_path}")
    print(f"レイアウト一覧:")
    for layout in doc.layouts:
        if layout.name != "Model":
            print(f"  - {layout.name}")


def main():
    parser = argparse.ArgumentParser(
        description="出来形管理図DXFにペーパースペースレイアウトを追加"
    )
    parser.add_argument("input", help="入力DXFファイル")
    parser.add_argument("--scale", "-s", type=float, required=True,
                        help="図面スケール（例: 50 = 1:50）")
    parser.add_argument("--pages", "-p", type=int, default=None,
                        help="ページ数（省略時は自動検出）")
    parser.add_argument("--output", "-o", type=str, default=None,
                        help="出力ファイル名（省略時は _with_layouts を付加）")

    args = parser.parse_args()

    add_layouts(args.input, args.scale, args.pages, args.output)


if __name__ == "__main__":
    main()
