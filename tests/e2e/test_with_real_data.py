import pandas as pd
import numpy as np
import os
from distance_utils import detect_distance_type, convert_to_cumulative_distance
import sys
import os
import sys
import os

# プロジェクトルートへのパスを追加
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))


# モジュールを条件付きでインポート
try:
    from src.distance_utils import detect_distance_type, convert_to_cumulative_distance
import sys
import os
except ImportError:
    try:
        from distance_utils import detect_distance_type, convert_to_cumulative_distance
import sys
import os
    except ImportError:
        print("警告: distance_utilsモジュールが見つかりません。テストはスキップされます。")



)


# モジュールを条件付きでインポート
try:
    from src.distance_utils import detect_distance_type, convert_to_cumulative_distance

def process_section_data
except ImportError:
    try:
        from distance_utils import detect_distance_type, convert_to_cumulative_distance

def process_section_data
    except ImportError:
        print("警告: distance_utilsモジュールが見つかりません。テストはスキップされます。")



def process_section_data(file_path, section_name):
    """CSVから区間データを抽出して処理する"""
    print(f"\n===== {section_name}のデータ処理 =====")
    
    # CSVファイル全体を読み込む
    print(f"ファイル '{file_path}' を読み込み中...")
    try:
        all_data = pd.read_csv(file_path, header=None)
        # 空白行や空白列を削除
        all_data = all_data.dropna(how='all').reset_index(drop=True)
        all_data = all_data.dropna(axis=1, how='all')
    except Exception as e:
        print(f"ファイル読み込みエラー: {e}")
        return None

    # セクション名が含まれる行を検索
    section_rows = all_data[all_data.apply(lambda row: any(str(cell) == section_name for cell in row if pd.notna(cell)), axis=1)]
    
    if section_rows.empty:
        print(f"{section_name}が見つかりませんでした")
        return None
    
    # セクション開始行のインデックス
    start_idx = section_rows.index[0]
    
    # ヘッダー行のインデックス（セクション名の次の行）
    header_idx = start_idx + 1
    
    if header_idx >= len(all_data):
        print(f"{section_name}のデータが見つかりませんでした")
        return None
    
    # ヘッダー行を取得
    headers = all_data.iloc[header_idx].tolist()
    
    # 必要な列のインデックスを特定
    name_col = None  # 測点名
    dist_col = None  # 単延長
    for i, header in enumerate(headers):
        if pd.notna(header) and '測点' in str(header):
            name_col = i
        elif pd.notna(header) and '単延長' in str(header):
            dist_col = i
    
    if name_col is None or dist_col is None:
        print("必要な列（測点名、単延長）が見つかりませんでした")
        return None
    
    # データ行の抽出（ヘッダーの次の行から次のセクションまで）
    data_start = header_idx + 1
    data_end = len(all_data)
    
    # 次のセクション行を探す
    for i in range(data_start, len(all_data)):
        if all_data.iloc[i].str.contains('区間').any():
            data_end = i
            break
    
    # データ行を抽出
    data_rows = all_data.iloc[data_start:data_end].copy()
    
    if data_rows.empty:
        print(f"{section_name}のデータ行が見つかりませんでした")
        return None
    
    # 必要な列のみ抽出
    try:
        section_data = pd.DataFrame({
            'name': data_rows.iloc[:, name_col],
            'x': data_rows.iloc[:, dist_col]
        })
        
        # 数値に変換できる行のみ残す
        section_data = section_data[section_data['x'].apply(lambda x: pd.notna(x) and str(x).replace('.', '', 1).isdigit())]
        section_data['x'] = section_data['x'].astype(float)
        
        # 測点名が空欄の場合、仮の名前を設定
        section_data['name'] = section_data['name'].fillna('測点')
        
        return section_data
    except Exception as e:
        print(f"データ処理エラー: {e}")
        return None

def main():
    file_path = '面積計算書、塩塚小野線側溝改修工事 - シート1.csv'
    sections = ['区間1', '区間3', '区間5', '区間6']
    
    if not os.path.exists(file_path):
        print(f"ファイル '{file_path}' が見つかりません")
        return
    
    print(f"ファイル '{file_path}' の処理を開始します\n")
    
    for section in sections:
        section_data = process_section_data(file_path, section)
        
        if section_data is not None and not section_data.empty:
            print(f"\n{section}のデータを抽出しました:")
            print(section_data)
            
            # 距離タイプの判別
            distance_type = detect_distance_type(section_data)
            print(f"\n距離タイプ判別結果: {distance_type}")
            
            # 単延長の場合は追加延長に変換
            if distance_type == 'tanenkyo':
                converted_data = convert_to_cumulative_distance(section_data)
                print("\n追加延長に変換しました:")
                print(converted_data)
                
                # 変換データの保存
                output_file = f"{section}_追加延長.csv"
                converted_data.to_csv(output_file, index=False)
                print(f"変換データを '{output_file}' に保存しました")
            else:
                print("既に追加延長形式のため、変換は不要です")
        
        print("\n" + "-"*50)

if __name__ == "__main__":
    main() 