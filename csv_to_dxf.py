import ezdxf
import pandas as pd
from ezdxf.enums import TextEntityAlignment
import pyperclip

def load_data(csv_path):
    """CSVファイルからデータを読み込む"""
    return pd.read_csv(csv_path)

def create_dxf_document():
    """新しいDXFドキュメントを作成する"""
    doc = ezdxf.new(dxfversion="R2010")
    return doc, doc.modelspace()

def draw_road_sections(msp, data):
    """道路断面と測点ラベルを描画する"""
    previous_left_point = None
    previous_right_point = None
    previous_center_point = None

    for index, row in data.iterrows():
        name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
        left_point = (x, wl)
        right_point = (x, -wr)
        center_point = (x, 0)

        # 幅員の線を描画
        msp.add_line(left_point, center_point)
        msp.add_line(center_point, right_point)

        # 測点ラベルを追加、-90度回転して上側に配置
        text = msp.add_text(name, dxfattribs={'height': 1.0, 'rotation': -90})
        text.dxf.insert = (x, max(wl, -wr) + 5)  # 高い方の上側に配置
        text.set_placement((x, max(wl, -wr) + 5), align=TextEntityAlignment.TOP_CENTER)
        # 外形線を描画
        if previous_left_point and previous_right_point:
            if left_point[1] > 0:
                msp.add_line(previous_left_point, left_point)
            if right_point[1] < 0:
                msp.add_line(previous_right_point, right_point)

        # センターラインを描画
        if previous_center_point:
            msp.add_line(previous_center_point, center_point)

        previous_left_point = left_point
        previous_right_point = right_point
        previous_center_point = center_point

def add_dimension_text(msp, text, position, rotation=0):
    """寸法テキストを追加する"""
    dimension_text = msp.add_text(text, dxfattribs={'height': 1.0, 'rotation': rotation})
    dimension_text.dxf.insert = position
    dimension_text.dxf.align_point = position
    dimension_text.set_placement(position, align=TextEntityAlignment.TOP_CENTER)

def draw_dimensions(msp, data):
    """幅員と延長の寸法を描画する"""
    for index, row in data.iterrows():
        name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
        # 左側の幅員寸法を描画
        if wl > 0.0:
            add_dimension_text(msp, f"{wl:.2f}", (x, wl / 2), rotation=-90)
        # 右側の幅員寸法を描画
        if wr > 0.0:
            add_dimension_text(msp, f"{wr:.2f}", (x, -wr / 2), rotation=-90)
        # 延長寸法を描画
        if index > 0:
            prev_x = data.iloc[index - 1]['x']
            if x - prev_x > 0.0:
                add_dimension_text(msp, f"{x - prev_x:.2f}", ((x + prev_x) / 2, 0))

def save_dxf_document(doc, dxf_path):
    """DXFドキュメントを保存する"""
    doc.saveas(dxf_path)
    print(f"DXF file has been saved as {dxf_path}")

def validate_data(data):
    if data.shape[1] != 4:
        raise ValueError("データの列数が正しくありません。4列のデータが必要です。")

    # 1列目が文字列、他の列が数値であることを確認
    if not all(data.iloc[:, 0].apply(lambda x: isinstance(x, str))):
        raise ValueError("1列目は全て文字列である必要があります。")

    #if not all(data.iloc[:, 1:].apply(lambda x: isinstance(x, (int, float)))):
     #   raise ValueError("2列目から4列目は全て数値である必要があります。")

def main():
    csv_path = 'data.csv'
    dxf_path = 'road_sections_with_names_and_dimensions.dxf'

    data = load_data(csv_path)
    doc, msp = create_dxf_document()

    draw_road_sections(msp, data)
    draw_dimensions(msp, data)

    save_dxf_document(doc, dxf_path)

if __name__ == "__main__":
    main()
