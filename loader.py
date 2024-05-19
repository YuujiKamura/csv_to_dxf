import pandas as pd
import tkinter as tk
from tkinter import messagebox, scrolledtext

def load_data_from_clipboard():
    # クリップボードからデータを読み込む
    try:
        data = pd.read_clipboard(header=None)
        data.columns = ["name", "x", "wl", "wr"]
        print("クリップボードから読み込んだデータ:")
        print(data)
        validate_data(data)
        return data
    except ValueError as e:
        print(f"データの検証エラー: {e}")
        return
    except Exception as e:
        print(f"予期しないエラーが発生しました: {e}")
        return

def load_data_from_csv():
    csv_path = 'data.csv'
    return pd.read_csv(csv_path)

def validate_data(data):
    if data.shape[1] != 4:
        message = "データの列数が正しくありません。4列のデータが必要です。"
        show_error_dialog(message)
        raise ValueError(message)

    # 1列目が文字列、他の列が数値であることを確認
    if not all(data.iloc[:, 0].apply(lambda x: isinstance(x, str))):
        message = "1列目は全て文字列である必要があります。"
        show_error_dialog(message)
        raise ValueError(message)

def show_error_dialog(message):
    root = tk.Tk()
    root.withdraw()  # ダイアログだけを表示するためにメインウィンドウを隠す
    messagebox.showerror("エラー", message)
    root.destroy()

def show_data_in_dialog(data, message):
    root = tk.Tk()
    root.title("クリップボードから読み込んだデータ")

    text_area = scrolledtext.ScrolledText(root, wrap=tk.WORD, width=60, height=20)
    text_area.pack(padx=10, pady=10, fill=tk.BOTH, expand=True)

    text_area.insert(tk.INSERT, data.to_string(index=False))
    text_area.insert(tk.INSERT, message)
    text_area.configure(state='disabled')  # Read-only

    root.mainloop()