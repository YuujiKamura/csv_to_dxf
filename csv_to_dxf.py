import ezdxf
import pandas as pd
import pyperclip
import dxf_draw_tenkaiz

def load_data(csv_path):
    """CSVファイルからデータを読み込む"""
    return pd.read_csv(csv_path)

def create_dxf_document():
    """新しいDXFドキュメントを作成する"""
    doc = ezdxf.new(dxfversion="R2010")
    return doc, doc.modelspace()

def save_dxf_document(doc, dxf_path):
    """DXFドキュメントを保存する"""
    doc.saveas(dxf_path)
    print(f"DXF file has been saved as {dxf_path}")

def main():
    csv_path = 'data.csv'
    dxf_path = 'road_sections_with_names_and_dimensions.dxf'

    data = load_data(csv_path)
    doc, msp = create_dxf_document()

    dxf_draw_tenkaiz.draw_road_sections(msp, data)

    save_dxf_document(doc, dxf_path)

if __name__ == "__main__":
    main()
