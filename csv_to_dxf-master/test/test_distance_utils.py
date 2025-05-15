import unittest
import pandas as pd
import numpy as np
from distance_utils import detect_distance_type, convert_to_cumulative_distance

class TestDistanceUtils(unittest.TestCase):
    
    def test_detect_distance_type_tanenkyo(self):
        """単延長データの判別テスト"""
        # 単延長データを作成（測点間の距離が増減するパターン）
        df = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2', 'No.3', 'No.4'],
            'x': [0.0, 10.0, 5.0, 15.0, 8.0],  # 単延長: 増減がある
            'wl': [3.0, 3.0, 3.5, 3.0, 3.0],
            'wr': [3.0, 3.0, 3.5, 3.0, 3.0]
        })
        
        result = detect_distance_type(df)
        self.assertEqual(result, 'tanenkyo', "単延長データを正しく判別できませんでした")
    
    def test_detect_distance_type_tsuikaenkyo(self):
        """追加延長データの判別テスト"""
        # 追加延長データを作成（測点間の距離が単調増加するパターン）
        df = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2', 'No.3', 'No.4'],
            'x': [0.0, 10.0, 20.0, 35.0, 50.0],  # 追加延長: 単調増加
            'wl': [3.0, 3.0, 3.5, 3.0, 3.0],
            'wr': [3.0, 3.0, 3.5, 3.0, 3.0]
        })
        
        result = detect_distance_type(df)
        self.assertEqual(result, 'tsuikaenkyo', "追加延長データを正しく判別できませんでした")
    
    def test_detect_distance_type_edge_cases(self):
        """エッジケースのテスト"""
        # 空のデータフレーム
        df_empty = pd.DataFrame()
        self.assertEqual(detect_distance_type(df_empty), 'unknown', "空のデータフレームを正しく処理できませんでした")
        
        # 1行のみのデータフレーム
        df_single = pd.DataFrame({'x': [10.0]})
        self.assertEqual(detect_distance_type(df_single), 'unknown', "1行のみのデータフレームを正しく処理できませんでした")
        
        # 列名が異なるデータフレーム
        df_diff_col = pd.DataFrame({'distance': [0.0, 10.0, 20.0]})
        self.assertEqual(detect_distance_type(df_diff_col), 'unknown', "列名が異なるデータフレームを正しく処理できませんでした")
        
        # カスタム列名を指定した場合
        self.assertEqual(detect_distance_type(df_diff_col, 'distance'), 'tsuikaenkyo', "カスタム列名を正しく処理できませんでした")
    
    def test_convert_to_cumulative_distance(self):
        """単延長から追加延長への変換テスト"""
        # 単延長データ
        df = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2', 'No.3'],
            'x': [5.0, 10.0, 15.0, 20.0],
            'wl': [3.0, 3.0, 3.5, 3.0]
        })
        
        # 変換結果
        result_df = convert_to_cumulative_distance(df)
        
        # 期待される結果
        expected = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2', 'No.3'],
            'x': [5.0, 15.0, 30.0, 50.0],  # 累積和: 5, 5+10, 5+10+15, 5+10+15+20
            'wl': [3.0, 3.0, 3.5, 3.0]
        })
        
        # DataFrameの比較
        pd.testing.assert_frame_equal(result_df, expected)
    
    def test_convert_with_nan_values(self):
        """NaN値を含むデータの変換テスト"""
        # NaN値を含む単延長データ
        df = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2', 'No.3'],
            'x': [5.0, np.nan, 15.0, 20.0],
            'wl': [3.0, 3.0, 3.5, 3.0]
        })
        
        # 変換結果
        result_df = convert_to_cumulative_distance(df)
        
        # NaNは累積和でもNaNとなることを確認
        self.assertTrue(np.isnan(result_df.iloc[1]['x']), "NaN値が正しく処理されていません")
        
        # NaN以外の値が正しく累積されることを確認
        self.assertEqual(result_df.iloc[0]['x'], 5.0)
        self.assertEqual(result_df.iloc[2]['x'], 20.0)  # 5 + NaN + 15 = NaNだが、pandasのcumsumはNaNをスキップする
        self.assertEqual(result_df.iloc[3]['x'], 40.0)  # 5 + NaN + 15 + 20 = 40
    
    def test_convert_custom_column(self):
        """カスタム列名での変換テスト"""
        # カスタム列名のデータフレーム
        df = pd.DataFrame({
            'point': ['A', 'B', 'C'],
            'distance': [10.0, 20.0, 30.0]
        })
        
        # カスタム列名で変換
        result_df = convert_to_cumulative_distance(df, 'distance')
        
        # 期待される結果
        expected = pd.DataFrame({
            'point': ['A', 'B', 'C'],
            'distance': [10.0, 30.0, 60.0]  # 累積和
        })
        
        # 結果の確認
        pd.testing.assert_frame_equal(result_df, expected)
    
    def test_real_world_example(self):
        """実際のデータに近いケースでのテスト"""
        # 測量データに近い形式のデータ
        df = pd.DataFrame({
            'name': ['No.0', '0+1.5', 'No.1', '1+5.0', 'No.2'],
            'x': [0.0, 1.5, 8.5, 5.0, 6.0],  # 単延長: 各点間の距離
            'wl': [2.5, 2.0, 2.0, 2.5, 3.0],
            'wr': [2.5, 2.0, 2.0, 2.5, 3.0]
        })
        
        # タイプ判別
        result_type = detect_distance_type(df)
        self.assertEqual(result_type, 'tanenkyo', "実データの単延長判別に失敗しました")
        
        # 変換
        converted_df = convert_to_cumulative_distance(df)
        
        # 期待される結果
        expected_distances = [0.0, 1.5, 10.0, 15.0, 21.0]  # 累積和
        
        # 結果の確認
        for i, expected in enumerate(expected_distances):
            self.assertAlmostEqual(converted_df.iloc[i]['x'], expected, 
                                 msg=f"累積距離の計算が正しくありません: インデックス {i}")

if __name__ == '__main__':
    unittest.main() 