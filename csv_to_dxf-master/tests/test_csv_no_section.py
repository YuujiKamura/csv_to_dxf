import unittest
import os
import sys
import pandas as pd

# テスト対象のモジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from gui.core.csv_processor import CSVProcessor

class TestNoSectionCSV(unittest.TestCase):
    """区間行がないCSVファイルのテスト"""
    
    def setUp(self):
        """テスト前の準備"""
        self.csv_processor = CSVProcessor()
        
        # テスト用のCSVファイルパス
        self.data_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'data'))
        self.csv_with_section = os.path.join(self.data_dir, "面積計算書、塩塚小野線側溝改修工事 - シート1.csv")
        self.csv_no_section = os.path.join(self.data_dir, "data.csv")
    
    def test_csv_with_section(self):
        """区間行ありのCSVファイルテスト"""
        # ファイルを読み込む
        result = self.csv_processor.read_csv_file(self.csv_with_section)
        self.assertTrue(result)
        
        # 区間を取得
        sections = self.csv_processor.get_available_sections()
        self.assertGreater(len(sections), 0)
        self.assertIn('区間1', sections)
        
        # 区間1を処理
        processed_data = self.csv_processor.process_section_data('区間1')
        self.assertIsNotNone(processed_data)
        self.assertIsInstance(processed_data, pd.DataFrame)
        self.assertGreater(len(processed_data), 0)
    
    def test_csv_no_section(self):
        """区間行なしのCSVファイルテスト"""
        # ファイルを読み込む
        result = self.csv_processor.read_csv_file(self.csv_no_section)
        self.assertTrue(result)
        
        # 区間を取得 - 単一区間として「区間1」が返されるはず
        sections = self.csv_processor.get_available_sections()
        self.assertEqual(len(sections), 1)
        self.assertEqual(sections[0], '区間1')
        
        # 区間1を処理
        processed_data = self.csv_processor.process_section_data('区間1')
        self.assertIsNotNone(processed_data)
        self.assertIsInstance(processed_data, pd.DataFrame)
        self.assertGreater(len(processed_data), 0)
        
        # データの内容を確認
        self.assertIn('name', processed_data.columns)
        self.assertIn('x', processed_data.columns)
        self.assertIn('wl', processed_data.columns)
        self.assertIn('wr', processed_data.columns)
        
        # 先頭行のデータを確認
        first_row = processed_data.iloc[0]
        self.assertEqual(first_row['name'], 'No.0')
        self.assertEqual(first_row['x'], 0.0)
        self.assertGreater(first_row['wl'], 0.0)
        self.assertGreater(first_row['wr'], 0.0)

if __name__ == '__main__':
    unittest.main() 