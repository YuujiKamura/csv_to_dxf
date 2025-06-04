import pandas as pd

from gui.core.csv_processor import CSVProcessor


def test_alias_apis(tmp_path):
    csv_path = tmp_path / "sample.csv"
    csv_content = "\n".join([
        "区間1,台形計算,,",
        "測点名,単延長L,幅員W,",
        "No.0,0,3.0,",
        "No.1,20,3.5,",
        "No.2,20,3.2,",
    ])
    csv_path.write_text(csv_content, encoding="utf-8")

    proc = CSVProcessor()
    assert proc.read_csv_file(csv_path)
    assert proc.file_path == str(csv_path)
    assert proc.csv_data is not None

    sec = proc.extract_section_data("区間1")
    assert isinstance(sec, pd.DataFrame)
    processed = proc.process_section_data("区間1")
    out_path = tmp_path / "processed.csv"
    assert proc.save_processed_data(processed, out_path)
    assert out_path.exists()
    df_saved = pd.read_csv(out_path)
    assert len(df_saved) == len(processed)

