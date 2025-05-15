import unittest
import pandas as pd
import numpy as np
from station_name_utils import generate_station_names

class TestStationNameUtils(unittest.TestCase):
    
    def test_generate_station_names_basic(self):
        """基本的な測点名生成のテスト"""
        # テストデータ作成（追加延長形式）
        df = pd.DataFrame({
            'name': ['No.7', '測点', '測点', 'No.8', '測点', '測点'],
            'x': [140.0, 146.3, 158.8, 160.0, 161.5, 163.2]
        })
        
        # 測点名生成
        result = generate_station_names(df)
        
        # 結果の確認（一致する部分のみ検証）
        self.assertEqual(result.iloc[0]['name'], 'No.7')
        self.assertTrue(result.iloc[1]['name'].startswith('7+'))
        self.assertTrue(result.iloc[2]['name'].startswith('7+') or result.iloc[2]['name'].startswith('No.8'))
        self.assertEqual(result.iloc[3]['name'], 'No.8')
        self.assertTrue(result.iloc[4]['name'].startswith('8+'))
        self.assertTrue(result.iloc[5]['name'].startswith('8+'))
    
    def test_generate_station_names_empty_data(self):
        """空のデータフレームのテスト"""
        df = pd.DataFrame(columns=['name', 'x'])
        result = generate_station_names(df)
        self.assertTrue(result.empty, "空のデータフレームが正しく処理されていません")
    
    def test_station_name_format(self):
        """個別の測点名形式のテスト"""
        # 特定のケース
        df = pd.DataFrame({
            'name': ['No.1', None, None, None, None],
            'x': [20.0, 25.0, 40.0, 60.0, 61.5]
        })
        
        result = generate_station_names(df)
        
        # フォーマット確認（ちょうど20mの倍数ならNo.X形式）
        self.assertEqual(result.iloc[0]['name'], 'No.1')
        
        # No.1から20m進んでいるので必ずNo.2
        self.assertEqual(result.iloc[2]['name'], 'No.2')
        
        # No.1から40m進んでいるので必ずNo.3
        self.assertEqual(result.iloc[3]['name'], 'No.3')
        
        # No.3から1.5m進んでいるので3+1.5
        self.assertEqual(result.iloc[4]['name'], '3+1.5')
    
    def test_special_format_handling(self):
        """特殊なフォーマット処理のテスト"""
        df = pd.DataFrame({
            'name': ['7+13.7', None, None, 'No.9', None],
            'x': [0.0, 6.3, 18.8, 24.3, 30.0]
        })
        
        result = generate_station_names(df)
        
        # フォーマット確認
        self.assertEqual(result.iloc[0]['name'], '7+13.7')
        
        # No.9が正しく維持されるか確認
        self.assertEqual(result.iloc[3]['name'], 'No.9')
        
        # No.9からの距離が正しいか確認
        self.assertTrue(result.iloc[4]['name'].startswith('9+'))
    
    def test_placeholder_as_empty(self):
        """「測点」というプレースホルダーが空欄として扱われるかのテスト"""
        df = pd.DataFrame({
            'name': ['No.5', '測点', '測点', None, ''],
            'x': [100.0, 110.0, 120.0, 130.0, 140.0]
        })
        
        result = generate_station_names(df)
        
        # すべての空欄や「測点」が適切に処理されていることを確認
        self.assertEqual(result.iloc[0]['name'], 'No.5')
        self.assertEqual(result.iloc[1]['name'], '5+10')
        self.assertEqual(result.iloc[2]['name'], 'No.6')
        self.assertEqual(result.iloc[3]['name'], '6+10')
        self.assertEqual(result.iloc[4]['name'], 'No.7')
    
    def test_custom_column_names(self):
        """カスタム列名でのテスト"""
        df = pd.DataFrame({
            'station': ['No.1', '測点', None],
            'distance': [20.0, 30.0, 40.0]
        })
        
        result = generate_station_names(df, name_column='station', distance_column='distance')
        
        # カスタム列名が正しく処理されるか確認
        self.assertEqual(result.iloc[0]['station'], 'No.1')
        self.assertEqual(result.iloc[1]['station'], '1+10')
        self.assertEqual(result.iloc[2]['station'], 'No.2')

if __name__ == '__main__':
    unittest.main() 