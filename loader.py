import pandas as pd
import tkinter as tk
from tkinter import messagebox, scrolledtext

def load_data_from_clipboard():
    # クリップボードからデータを読み込む
    try:
        data = pd.read_clipboard(header=None)
        data.columns = ["name", "x", "wl", "wr"]
        return data
    except pd.errors.ParserError as e:
        show_error_dialog("クリップボードからデータを読み込めませんでした。データの形式を確認してください。")
        print(f"パースエラー: {e}")
        return None
    except Exception as e:
        show_error_dialog("クリップボードからデータを読み込む際に予期しないエラーが発生しました。")
        print(f"予期しないエラー: {e}")
        return None

def load_data_from_csv():
    csv_path = 'data.csv'
    return pd.read_csv(csv_path)

def show_usage():
    sample_data = [
        ["測点", "追加距離", "左幅員", "右幅員"],
        ["No.0", f"{0}", f"{3.50}", f"{3.35}"],
        ["No.1", f"{10}", f"{3.40}", f"{3.35}"]
    ]
    # 各列を適切にタブ区切りで整形
    sample_data_str = "\n".join(["\t".join(row) for row in sample_data])

    message = ("\n\n<<つかいかた>>\n"
               "excelなどの表計算ソフトを使って４列構成の表を作成し、\n"
               "クリップボードにコピーした後に実行してください。\n\n"
               f"{sample_data_str}\n\n"
               "ヘッダー行は入力不要です\n"
               "成功すると、同一ディレクトリにdxfファイルが作成されます。\n\n"
               )
    return message

def validate_data(data):
    message = show_usage()
    conditions = [
        (data.shape[1] != 4, "データは4列構成でなければなりません。"),
        (not all(isinstance(x, str) for x in data.iloc[:, 0]), "1列目は文字列でなければなりません。"),
    ]

    errors = [msg for condition, msg in conditions if condition]

    if errors:
        show_data_in_dialog(data, message)
        error_message = "\n".join(errors)
        show_error_dialog(f"{message}\n{error_message}")
        print(f"入力エラー:\n{error_message}")
        print("クリップボードから読み込んだデータ:")
        print(data)
        return None

    return data

def show_error_dialog(message):
    root = tk.Tk()
    root.withdraw()  # ダイアログだけを表示するためにメインウィンドウを隠す
    messagebox.showerror("エラー", message)
    root.destroy()

def show_data_in_dialog(data, message):
    root = tk.Tk()
    root.title("road_dxf_drawer")

    text_area = scrolledtext.ScrolledText(root, wrap=tk.WORD, width=60, height=20)
    text_area.pack(padx=10, pady=10, fill=tk.BOTH, expand=True)

    text_area.insert(tk.INSERT, message)
    text_area.insert(tk.INSERT, data.to_string(index=False))
    text_area.configure(state='disabled')  # Read-only

    root.mainloop()