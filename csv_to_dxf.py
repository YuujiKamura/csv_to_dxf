import ezdxf
import pandas as pd
import pyperclip
import dxf_draw_tenkaiz

def main():
    csv_path = 'data.csv'
    dxf_path = 'road_sections_with_names_and_dimensions.dxf'
    data = pd.read_csv(csv_path)
    doc  = ezdxf.new(dxfversion="R2010")
    dxf_draw_tenkaiz.draw_road_sections( doc.modelspace(), data )
    doc.saveas(dxf_path)
    print(f"DXF file has been saved as {dxf_path}")

if __name__ == "__main__":
    main()
