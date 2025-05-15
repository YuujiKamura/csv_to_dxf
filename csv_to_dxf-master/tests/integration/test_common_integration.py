import pandas as pd
import numpy as np
import os
from csv_processing import process_csv_for_dxf, extract_section_data
from distance_utils import detect_distance_type, convert_to_cumulative_distance
from station_name_utils import generate_station_names
import sys
import os
import sys
import os

# プロジェクトルートへのパスを追加
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))


# モジュールを条件付きでインポート
try:
    from src.csv_processing import process_csv_for_dxf, extract_section_data
from distance_utils import detect_distance_type, convert_to_cumulative_distance
from station_name_utils import generate_station_names
import sys
import os
except ImportError:
    try:
        from csv_processing import process_csv_for_dxf, extract_section_data
from distance_utils import detect_distance_type, convert_to_cumulative_distance
from station_name_utils import generate_station_names
import sys
import os
    except ImportError:
        print("警告: csv_processingモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    from src.distance_utils import detect_distance_type, convert_to_cumulative_distance
from station_name_utils import generate_station_names

def test_full_processing
    except ImportError
except ImportError:
    try:
        from distance_utils import detect_distance_type, convert_to_cumulative_distance
from station_name_utils import generate_station_names

def test_full_processing
    except ImportError
    except ImportError:
        print("警告: distance_utilsモジュールが見つかりません。テストはスキップされます。")



)


# モジュールを条件付きでインポート
try:
    from src.csv_processing import process_csv_for_dxf, extract_section_data
from distance_utils import detect_distance_type, convert_to_cumulative_distance
from station_name_utils import generate_station_names

def test_full_processing
except ImportError:
    try:
        from csv_processing import process_csv_for_dxf, extract_section_data
from distance_utils import detect_distance_type, convert_to_cumulative_distance
from station_name_utils import generate_station_names

def test_full_processing
    except ImportError:
        print("警告: csv_processingモジュールが見つかりません。テストはスキップされます。")



def test_full_processing():
    """距離判別・追加延長変換・測点名生成の統合テスト"""
    file_path = '面積計算書、塩塚小野線側溝改修工事 - シート1.csv'
    
    if not os.path.exists(file_path):
        print(f"テストファイル '{file_path}' が見つかりません")
        return
    
    print(f"ファイル '{file_path}' を使用した統合テストを開始します\n")
    
    # 区間データを抽出
    section_name = '区間6'  # テスト用区間
    section_data = extract_section_data(file_path, section_name)
    
    if section_data is None or section_data.empty:
        print(f"{section_name}のデータが抽出できませんでした")
        return
    
    print(f"\n1. 抽出された{section_name}のデータ:")
    print(section_data)
    
    # 距離タイプの判別
    distance_type = detect_distance_type(section_data)
    print(f"\n2. 距離タイプの判別結果: {distance_type}")
    
    # 単延長から追加延長への変換（必要な場合）
    if distance_type == 'tanenkyo':
        converted_data = convert_to_cumulative_distance(section_data)
        print("\n3. 追加延長に変換されたデータ:")
        print(converted_data)
    else:
        converted_data = section_data
        print("\n3. データは既に追加延長形式です")
    
    # 測点名の生成
    try:
        # 測点名を生成（元のデータのコピーを作成して使用）
        data_for_naming = converted_data.copy()
        named_data = generate_station_names(data_for_naming)
        print("\n4. 測点名が生成されたデータ:")
        print(named_data)
        
        # 測点名の変換結果を表示
        print("\n5. 測点名の変換結果:")
        
        # 比較用のデータフレームを作成
        comparison_data = []
        for i, row in named_data.iterrows():
            old_name = converted_data.loc[i, 'name'] if i in converted_data.index else None
            new_name = row['name']
            
            # すべての名前変換を表示（元の名前と新しい名前が違う場合）
            if old_name != new_name:
                comparison_data.append({
                    '行番号': i,
                    '距離': row['x'],
                    '元の名前': '測点' if str(old_name) == '測点' else str(old_name),
                    '新しい測点名': new_name
                })
        
        # 比較結果を表示
        if comparison_data:
            comparison_df = pd.DataFrame(comparison_data)
            print(comparison_df)
        else:
            print("測点名の変換はありませんでした")
            
        # すべての測点名を表示（テーブル形式）
        print("\n6. すべての測点名一覧:")
        station_names_data = []
        for i, row in named_data.iterrows():
            station_names_data.append({
                '行番号': i,
                '測点名': row['name'],
                '距離': row['x']
            })
        
        station_names_df = pd.DataFrame(station_names_data)
        print(station_names_df)
    
    except Exception as e:
        print(f"測点名生成中にエラーが発生しました: {e}")
    
    print("\n統合テストが完了しました")

if __name__ == "__main__":
    test_full_processing() 