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
    prev_points = (None,None,None)

    for index, row in data.iterrows():
        name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
        left_point = (x, wl)
        right_point = (x, -wr)
        center_point = (x, 0)

        draw_matomete_lines(left_point, center_point, right_point, prev_points, msp)
        draw_sokuten(msp, name, wl, wr, x)

        prev_points = ( left_point, center_point, right_point )

def draw_matomete_lines(left_point, center_point, right_point, prev_points, msp):
    # 幅員の線を描画
    msp.add_line(left_point, center_point)
    msp.add_line(center_point, right_point)
    draw_gaikeisen(left_point, msp, prev_points[0], prev_points[2], right_point)
    draw_centerline(center_point, msp, prev_points[1])

def draw_centerline(center_point, msp, previous_center_point):
    # センターラインを描画
    if previous_center_point:
        msp.add_line(previous_center_point, center_point)

def draw_gaikeisen(left_point, msp, previous_left_point, previous_right_point, right_point):
    # 外形線を描画
    if previous_left_point and previous_right_point:
        if left_point[1] > 0:
            msp.add_line(previous_left_point, left_point)
        if right_point[1] < 0:
            msp.add_line(previous_right_point, right_point)

def draw_all_dimensions(msp, data):
    """幅員と延長の寸法を描画する"""
    for index, row in data.iterrows():
        draw_set_of_dimensions(data, index, msp, row)

def draw_set_of_dimensions(data, index, msp, row):
    name, x, wl, wr = row['name'], row['x'], row['wl'], row['wr']
    prev_x = 0
    if index > 0:
        prev_x = data.iloc[index - 1]['x']
    # 延長寸法を描画
    if x - prev_x > 0.0:
        add_dimension_text(msp, f"{x - prev_x:.2f}", ((x + prev_x) / 2, 0))
    alignment = TOP_CENTER
    if x - prev_x < 1.0:
        alignment = BOTTOM_CENTER
    # 左側の幅員寸法を描画
    draw_positive_dimension(msp, wl, (x, wl / 2), -90, alignment)
    # 右側の幅員寸法を描画
    draw_positive_dimension(msp, wr, (x, -wr / 2), -90, alignment)

TOP_CENTER=TextEntityAlignment.TOP_CENTER
BOTTOM_CENTER=TextEntityAlignment.BOTTOM_CENTER

def draw_positive_dimension(msp, value, position, rotation, alignment=TOP_CENTER):
    if value > 0.0:
        add_dimension_text(msp, f"{value:.2f}", position, rotation, alignment )

def add_dimension_text(msp, text, position, rotation=0, alignment=TOP_CENTER):
    """寸法テキストを追加する"""
    dimension_text = msp.add_text(text, dxfattribs={'height': 1.0, 'rotation': rotation})
    dimension_text.dxf.insert = position
    dimension_text.dxf.align_point = position
    dimension_text.set_placement(position, align=alignment)

def draw_sokuten(msp, name, wl, wr, x, alignment=TOP_CENTER):
    # 測点ラベルを追加、-90度回転して上側に配置
    text = msp.add_text(name, dxfattribs={'height': 1.0, 'rotation': -90})
    text.dxf.insert = (x, max(wl, -wr) + 5)  # 高い方の上側に配置
    text.set_placement((x, max(wl, -wr) + 5), align=alignment)

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
    draw_all_dimensions(msp, data)

    save_dxf_document(doc, dxf_path)

if __name__ == "__main__":
    main()
