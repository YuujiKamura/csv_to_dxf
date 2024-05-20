import ezdxf
import loader
import dxf_draw_tenkaiz


def main():
    dxf_path = 'road_sections.dxf'

    # Load and validate data
    data = loader.load_data_from_clipboard()
    data = loader.validate_data(data)
    if data is None:
        return

    # Create a new DXF document
    doc = ezdxf.new(dxfversion="R2010")

    # Set the units to meters
    doc.header['$INSUNITS'] = 6  # 6 is the code for meters
    doc.header['$MEASUREMENT'] = 1  # 1 is the code for metric units

    # Create modelspace and draw road sections
    dxf_draw_tenkaiz.draw_road_sections(doc.modelspace(), data)

    # Save the DXF document
    doc.saveas(dxf_path)

    # Notify user of the saved file
    message = f"\nDXF file has been saved as {dxf_path}\n"
    loader.show_data_in_dialog(data, message)
    print(message)

if __name__ == "__main__":
    main()
