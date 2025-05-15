import unittest
import os
import sys
import pandas as pd
import tempfile

# テスト対象のモジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from gui.core.csv_processor import CSVProcessor

class TestCSVProcessor(unittest.TestCase):
    """CSVProcessorクラスのテスト"""
    
    def setUp(self):
        """テスト前の準備"""
        self.processor = CSVProcessor()
        
        # テスト用の一時ファイルを作成
        self.test_csv = self.create_test_csv()
    
    def tearDown(self):
        """テスト後のクリーンアップ"""
        # 一時ファイルを削除
        if hasattr(self, 'test_csv') and os.path.exists(self.test_csv):
            os.remove(self.test_csv)
    
    def create_test_csv(self):
        """テスト用のCSVファイルを作成"""
        # 一時ファイルを作成
        fd, path = tempfile.mkstemp(suffix='.csv')
        os.close(fd)
        
        # テスト用のデータを書き込む
        with open(path, 'w', encoding='utf-8') as f:
            f.write("区間1,,,\n")
            f.write("測点,単延長L,幅員W,\n")
            f.write("No.0,0,3.0,\n")
            f.write("No.1,20,3.5,\n")
            f.write("No.2,20,4.0,\n")
            f.write("No.3,20,3.8,\n")
            f.write("区間2,,,\n")
            f.write("測点,単延長L,幅員W,\n")
            f.write("No.0,0,2.5,\n")
            f.write("No.1,20,2.7,\n")
        
        return path
    
    def test_read_csv_file(self):
        """ファイル読み込みのテスト"""
        # 存在するファイルを読み込む
        result = self.processor.read_csv_file(self.test_csv)
        self.assertTrue(result)
        self.assertEqual(self.processor.file_path, self.test_csv)
        self.assertIsNotNone(self.processor.csv_data)
        
        # 存在しないファイルを読み込む
        result = self.processor.read_csv_file("non_existent_file.csv")
        self.assertFalse(result)
    
    def test_get_available_sections(self):
        """利用可能な区間の取得テスト"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.test_csv)
        
        # 区間を取得
        sections = self.processor.get_available_sections()
        
        # 結果を検証
        self.assertIsInstance(sections, list)
        self.assertEqual(len(sections), 2)
        self.assertIn('区間1', sections)
        self.assertIn('区間2', sections)
    
    def test_extract_section_data(self):
        """区間データ抽出のテスト"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.test_csv)
        
        # 区間1のデータを抽出
        result = self.processor.extract_section_data('区間1')
        
        # 結果を検証
        self.assertIsInstance(result, pd.DataFrame)
        self.assertEqual(len(result), 4)  # 4行のデータ
        self.assertTrue('name' in result.columns)
        self.assertTrue('x' in result.columns)
        self.assertTrue('wl' in result.columns)
        self.assertTrue('wr' in result.columns)
        
        # 存在しない区間の抽出
        result = self.processor.extract_section_data('存在しない区間')
        self.assertIsNone(result)
    
    def test_process_section_data(self):
        """区間データ処理のテスト"""
        # ファイルを読み込む
        self.processor.read_csv_file(self.test_csv)
        
        # 区間1のデータを処理
        result = self.processor.process_section_data('区間1')
        
        # 結果を検証
        self.assertIsInstance(result, pd.DataFrame)
        self.assertEqual(len(result), 4)  # 4行のデータ
        
        # 単延長から追加延長への変換を確認
        # 累積値になっているはず: 0, 20, 40, 60
        self.assertEqual(result['x'].iloc[0], 0)
        self.assertEqual(result['x'].iloc[1], 20)
        self.assertEqual(result['x'].iloc[2], 40)
        self.assertEqual(result['x'].iloc[3], 60)
        
        # 測点名の正規化を確認
        self.assertEqual(result['name'].iloc[0], "No.0")
        self.assertEqual(result['name'].iloc[1], "No.1")
        self.assertEqual(result['name'].iloc[2], "No.2")
        self.assertEqual(result['name'].iloc[3], "No.3")
    
    def test_save_processed_data(self):
        """処理データの保存テスト"""
        # ファイルを読み込み、処理する
        self.processor.read_csv_file(self.test_csv)
        processed_data = self.processor.process_section_data('区間1')
        
        # 一時ファイルに保存
        fd, output_path = tempfile.mkstemp(suffix='.csv')
        os.close(fd)
        
        try:
            # 保存を実行
            result = self.processor.save_processed_data(processed_data, output_path)
            
            # 結果を検証
            self.assertTrue(result)
            self.assertTrue(os.path.exists(output_path))
            
            # 保存されたファイルを読み込んで検証
            saved_df = pd.read_csv(output_path)
            self.assertEqual(len(saved_df), 4)
            self.assertTrue('name' in saved_df.columns)
            self.assertTrue('x' in saved_df.columns)
            self.assertTrue('wl' in saved_df.columns)
            self.assertTrue('wr' in saved_df.columns)
        finally:
            # 出力ファイルを削除
            if os.path.exists(output_path):
                os.remove(output_path)

if __name__ == '__main__':
    unittest.main() 