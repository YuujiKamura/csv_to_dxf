"""
ezdxfでレイアウト追加のテスト
A3図枠2枚を縦に並べて、それぞれにペーパースペースレイアウトを付ける
"""
import ezdxf
from ezdxf.enums import TextEntityAlignment

# A3サイズ (mm)
A3_WIDTH = 420
A3_HEIGHT = 297

# スケール 1:100
SCALE = 100

# DXF単位でのA3サイズ
FRAME_WIDTH = A3_WIDTH * SCALE  # 42000
FRAME_HEIGHT = A3_HEIGHT * SCALE  # 29700

# 図枠間の間隔
GAP = 5000

def draw_frame(msp, x_offset, y_offset, page_num):
    """A3図枠を描画"""
    x1, y1 = x_offset, y_offset
    x2, y2 = x_offset + FRAME_WIDTH, y_offset + FRAME_HEIGHT

    # 外枠
    msp.add_line((x1, y1), (x2, y1))
    msp.add_line((x2, y1), (x2, y2))
    msp.add_line((x2, y2), (x1, y2))
    msp.add_line((x1, y2), (x1, y1))

    # 内枠 (10mm = 1000 DXF単位)
    margin = 10 * SCALE
    ix1, iy1 = x1 + margin, y1 + margin
    ix2, iy2 = x2 - margin, y2 - margin
    msp.add_line((ix1, iy1), (ix2, iy1))
    msp.add_line((ix2, iy1), (ix2, iy2))
    msp.add_line((ix2, iy2), (ix1, iy2))
    msp.add_line((ix1, iy2), (ix1, iy1))

    # ページ番号テキスト
    center_x = (x1 + x2) / 2
    center_y = (y1 + y2) / 2
    msp.add_text(
        f"Page {page_num}",
        height=3000,
        dxfattribs={"insert": (center_x, center_y)}
    )

def main():
    # DXF R2000形式で新規作成（レイアウト機能に必要）
    doc = ezdxf.new("R2000")
    msp = doc.modelspace()

    # 2枚の図枠を縦に並べて描画
    page_positions = []

    for i in range(2):
        y_offset = i * (FRAME_HEIGHT + GAP)
        draw_frame(msp, 0, y_offset, i + 1)
        # 各ページの中心座標を記録（モデル空間）
        page_positions.append({
            "center_x": FRAME_WIDTH / 2,
            "center_y": y_offset + FRAME_HEIGHT / 2,
            "page_num": i + 1
        })

    # ビューポート用レイヤー（非表示にすると枠が消える）
    vp_layer = doc.layers.add("VIEWPORTS")
    # vp_layer.off()  # 枠を非表示にしたい場合

    # 各ページ用のペーパースペースレイアウトを作成
    for page in page_positions:
        layout_name = f"A3_Page{page['page_num']}"
        layout = doc.layouts.new(layout_name)

        # A3横向きのページ設定
        layout.page_setup(
            size=(A3_WIDTH, A3_HEIGHT),
            margins=(0, 0, 0, 0),  # マージンなし
            units="mm"
        )

        # ビューポート追加
        # 1:100スケールで図枠全体を表示
        # view_height: モデル空間での表示高さ（DXF単位）
        # ペーパー空間でのビューポート高さ = view_height / scale
        # つまり FRAME_HEIGHT / 100 = 297mm
        vp = layout.add_viewport(
            center=(A3_WIDTH / 2, A3_HEIGHT / 2),  # 用紙中央 (210, 148.5)
            size=(A3_WIDTH, A3_HEIGHT),            # ビューポートサイズ = 用紙サイズ
            view_center_point=(page["center_x"], page["center_y"], 0),
            view_height=FRAME_HEIGHT,              # 29700 DXF単位 → 297mm at 1:100
            dxfattribs={"layer": "VIEWPORTS"}
        )

    # デフォルトのLayout1を削除
    if "Layout1" in doc.layouts:
        doc.layouts.delete("Layout1")

    # 保存
    output_path = "test_layout_output.dxf"
    doc.saveas(output_path)
    print(f"保存完了: {output_path}")
    print(f"レイアウト数: {len(doc.layouts)}")
    for layout in doc.layouts:
        print(f"  - {layout.name}")

if __name__ == "__main__":
    main()
