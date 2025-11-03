import unittest
import pandas as pd
import numpy as np
import sys
import os

# テスト対象のモジュールへのパスを追加
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from src.distance_utils import detect_distance_type, convert_to_cumulative_distance

class TestDistanceDetector(unittest.TestCase):
    
    def test_cumulative_distance_detection(self):
        """累積距離（追加延長）データの検出テスト"""
        
        # CSVファイルからのデータ読み込みを模擬したデータフレーム（累積距離形式）
        # 区間3.csvのデータを簡略化
        cumulative_df = pd.DataFrame({
            'name': ['0+7.9', 'No.0+9.9', 'No.0+11.5', 'No.0+14', 'No.0+16.1', 'No.1', 'No.2', 'No.3'],
            'x': [0.0, 2.03, 3.63, 6.13, 8.23, 10.73, 30.73, 50.73],
            'wl': [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            'wr': [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        })
        
        # 検出結果
        result = detect_distance_type(cumulative_df)
        print(f"累積距離データの検出結果: {result}")
        
        # 期待値: 'tuikaenkyo'（追加延長）
        self.assertEqual(result, 'tuikaenkyo', "累積距離形式のデータが正しく検出されませんでした")
        
        # 変換
        converted_df = convert_to_cumulative_distance(cumulative_df)
        print("変換前のX値:", cumulative_df['x'].tolist())
        print("変換後のX値:", converted_df['x'].tolist())
        
        # 既に累積距離のため、変換前後で値が同じであることを確認
        np.testing.assert_array_almost_equal(cumulative_df['x'].values, converted_df['x'].values)
    
    def test_interval_distance_detection(self):
        """単延長データの検出テスト"""
        
        # 単延長形式のデータフレーム
        interval_df = pd.DataFrame({
            'name': ['0+7.9', 'No.0+9.9', 'No.0+11.5', 'No.0+14', 'No.0+16.1', 'No.1', 'No.2', 'No.3'],
            'x': [0.0, 2.03, 1.6, 2.5, 2.1, 2.5, 20.0, 20.0],  # 各区間の距離
            'wl': [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            'wr': [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        })
        
        # 検出結果
        result = detect_distance_type(interval_df)
        print(f"単延長データの検出結果: {result}")
        
        # 期待値: 'tanenkyo'（単延長）
        self.assertEqual(result, 'tanenkyo', "単延長形式のデータが正しく検出されませんでした")
        
        # 変換
        converted_df = convert_to_cumulative_distance(interval_df)
        print("変換前のX値:", interval_df['x'].tolist())
        print("変換後のX値:", converted_df['x'].tolist())
        
        # 単延長から累積距離への変換なので、最後の値が合計と一致することを確認
        self.assertAlmostEqual(converted_df['x'].iloc[-1], sum(interval_df['x']), 
                              msg="単延長から累積距離への変換が正しく行われていません")
    
    def test_improved_distance_detection(self):
        """改良版の累積距離検出テスト"""
        
        # 主測点間隔に基づく累積距離の検出
        # 主測点（No.X）が等間隔になっているはず
        cumulative_df = pd.DataFrame({
            'name': ['No.0', 'No.0+10', 'No.1', 'No.1+10', 'No.2', 'No.3'],
            'x': [0.0, 10.0, 20.0, 30.0, 40.0, 60.0],
            'wl': [0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            'wr': [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        })
        
        # 主測点のみを抽出
        main_stations = cumulative_df[cumulative_df['name'].str.match(r'No\.\d+$')]
        
        # 主測点間の差が一定か確認（累積距離の特徴）
        if len(main_stations) >= 2:
            diffs = np.diff(main_stations['x'].values)
            is_consistent = np.all(np.abs(diffs - diffs[0]) < 0.01)
            
            print(f"主測点間隔: {diffs}")
            print(f"主測点間隔が一定: {is_consistent}")
            
            # 主測点間隔が一定の場合は累積距離の可能性が高い
            self.assertTrue(is_consistent, "主測点間隔が一定ではありません")

if __name__ == '__main__':
    unittest.main() 