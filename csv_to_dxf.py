import ezdxf
import loader
import dxf_draw_tenkaiz

def main():
    dxf_path = 'road_sections.dxf'

    data = loader.load_data_from_clipboard()
    doc  = ezdxf.new(dxfversion="R2010")

    dxf_draw_tenkaiz.draw_road_sections( doc.modelspace(), data )

    doc.saveas(dxf_path)
    message = f"\nDXF file has been saved as {dxf_path}\n"
    loader.show_data_in_dialog(data, message)
    print(message)

if __name__ == "__main__":
    main()
