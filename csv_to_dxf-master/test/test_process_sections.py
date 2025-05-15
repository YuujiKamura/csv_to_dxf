import unittest
import os
import pandas as pd
import numpy as np
import tempfile
import shutil
from process_sections import extract_all_sections, process_section_data, process_all_sections_pure, save_sections_to_csv

class TestProcessSections(unittest.TestCase):
    
    def setUp(self):
        """テスト用の一時ディレクトリとサンプルCSVファイルを作成"""
        # 一時ディレクトリの作成
        self.test_dir = tempfile.mkdtemp()
        
        # サンプルCSVの内容
        self.sample_csv_content = """,,,,
区間1,台形計算,,,
測点名,単延長L,幅員W,平均幅員Wa,面積m2
No.0,0,0.8,,
,1.15,0.63,0.715,0.82

区間2,台形計算,,,
測点名,単延長L,幅員W,平均幅員Wa,面積m2
No.5,0,0.5,,
,2.0,0.5,0.5,1.0
"""
        
        # サンプルCSVファイルの作成
        self.sample_csv_path = os.path.join(self.test_dir, "sample.csv")
        with open(self.sample_csv_path, 'w', encoding='utf-8') as f:
            f.write(self.sample_csv_content)
    
    def tearDown(self):
        """テスト用の一時ディレクトリの削除"""
        shutil.rmtree(self.test_dir)
    
    def test_extract_all_sections(self):
        """区間抽出の機能をテスト"""
        # 区間の抽出
        sections = extract_all_sections(self.sample_csv_path)
        
        # 結果の検証
        self.assertEqual(len(sections), 2)
        self.assertIn("区間1", sections)
        self.assertIn("区間2", sections)
    
    def test_extract_all_sections_nonexistent_file(self):
        """存在しないファイルに対する挙動をテスト"""
        # 存在しないファイルからの抽出
        sections = extract_all_sections("nonexistent.csv")
        
        # 空のリストが返されることを確認
        self.assertEqual(sections, [])
    
    def test_process_section_data(self):
        """区間データ処理の機能をテスト"""
        # 区間1のデータを処理
        result = process_section_data(self.sample_csv_path, "区間1")
        
        # データが取得できたことを確認
        self.assertIsNotNone(result)
        self.assertIsInstance(result, pd.DataFrame)
        
        # 行数の確認
        self.assertTrue(len(result) > 0)
        
        # wlとwr列の値を確認
        for i, row in result.iterrows():
            # wrが0になっていることを確認
            self.assertEqual(row['wr'], 0.0)
    
    def test_process_section_data_nonexistent_section(self):
        """存在しない区間に対する挙動をテスト"""
        # 存在しない区間を処理
        result = process_section_data(self.sample_csv_path, "区間999")
        
        # Noneが返されることを確認
        self.assertIsNone(result)
    
    def test_process_all_sections_pure(self):
        """全区間処理の機能をテスト"""
        # すべての区間を処理
        results = process_all_sections_pure(self.sample_csv_path)
        
        # 結果の検証
        self.assertEqual(len(results), 2)
        self.assertIn("区間1", results)
        self.assertIn("区間2", results)
        
        # 各区間のデータを確認
        for section_name, data in results.items():
            self.assertIsInstance(data, pd.DataFrame)
            self.assertTrue(len(data) > 0)
            
            # wrが0になっていることを確認
            for i, row in data.iterrows():
                self.assertEqual(row['wr'], 0.0)
    
    def test_save_sections_to_csv(self):
        """CSVファイル保存の機能をテスト"""
        # テスト用のデータフレームを作成
        df1 = pd.DataFrame({
            'name': ['No.1', 'No.2'],
            'x': [0.0, 20.0],
            'wl': [0.5, 0.5],
            'wr': [0.0, 0.0]
        })
        
        df2 = pd.DataFrame({
            'name': ['No.3', 'No.4'],
            'x': [0.0, 20.0],
            'wl': [0.6, 0.6],
            'wr': [0.0, 0.0]
        })
        
        # 辞書を作成
        sections_data = {
            "区間A": df1,
            "区間B": df2
        }
        
        # 一時ディレクトリに保存
        saved_files = save_sections_to_csv(sections_data, self.test_dir)
        
        # 保存されたファイル数を確認
        self.assertEqual(len(saved_files), 2)
        
        # 各ファイルが存在することを確認
        for file_path in saved_files:
            self.assertTrue(os.path.exists(file_path))
        
        # 保存されたファイルの内容を確認
        for section_name, df in sections_data.items():
            file_path = os.path.join(self.test_dir, f"{section_name}.csv")
            
            # ファイルを読み込み
            read_df = pd.read_csv(file_path)
            
            # 行数が一致することを確認
            self.assertEqual(len(read_df), len(df))
            
            # wl列とwr列の値を確認
            for i, row in read_df.iterrows():
                # wrが0になっていることを確認
                self.assertEqual(row['wr'], 0.0)

if __name__ == '__main__':
    unittest.main() 