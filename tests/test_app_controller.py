import unittest
import os
import sys
import pandas as pd
import tempfile

# テスト対象のモジュールへのパスを追加
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from gui.app_controller import AppController

class TestAppController(unittest.TestCase):
    """AppControllerクラスのテスト"""
    
    def setUp(self):
        """テスト前の準備"""
        self.controller = AppController()
        
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
    
    def test_load_file(self):
        """ファイル読み込みのテスト"""
        # ファイルを読み込む
        result = self.controller.load_file(self.test_csv)
        
        # 結果を検証
        self.assertTrue(result)
        self.assertEqual(self.controller.get_current_file_path(), self.test_csv)
    
    def test_get_available_sections(self):
        """利用可能な区間取得のテスト"""
        # ファイルを読み込む
        self.controller.load_file(self.test_csv)
        
        # 区間を取得
        sections = self.controller.get_available_sections()
        
        # 結果を検証
        self.assertIsInstance(sections, list)
        self.assertEqual(len(sections), 2)
        self.assertIn('区間1', sections)
        self.assertIn('区間2', sections)
    
    def test_process_section(self):
        """区間処理のテスト"""
        # ファイルを読み込む
        self.controller.load_file(self.test_csv)
        
        # 区間1を処理
        result = self.controller.process_section('区間1')
        
        # 結果を検証
        self.assertIsInstance(result, pd.DataFrame)
        self.assertTrue(self.controller.has_processed_data())
        self.assertEqual(len(result), 4)  # 4行のデータがあるはず
    
    def test_convert_to_dxf(self):
        """DXF変換のテスト"""
        # ファイルを読み込む
        self.controller.load_file(self.test_csv)
        
        # 区間1を処理
        self.controller.process_section('区間1')
        
        # DXFに変換
        fd, output_path = tempfile.mkstemp(suffix='.dxf')
        os.close(fd)
        
        try:
            # 変換実行
            result = self.controller.convert_to_dxf(output_path)
            
            # 結果を検証
            self.assertTrue(result)
            self.assertTrue(os.path.exists(output_path))
            self.assertGreater(os.path.getsize(output_path), 0)
        finally:
            # 出力ファイルを削除
            if os.path.exists(output_path):
                os.remove(output_path)
    
    def test_save_as_csv(self):
        """CSV保存のテスト"""
        # ファイルを読み込む
        self.controller.load_file(self.test_csv)
        
        # 区間1を処理
        self.controller.process_section('区間1')
        
        # CSVに保存
        fd, output_path = tempfile.mkstemp(suffix='.csv')
        os.close(fd)
        
        try:
            # 保存実行
            result = self.controller.save_as_csv(output_path)
            
            # 結果を検証
            self.assertTrue(result)
            self.assertTrue(os.path.exists(output_path))
            self.assertGreater(os.path.getsize(output_path), 0)
            
            # 保存されたCSVを読み込んで検証
            df = pd.read_csv(output_path)
            self.assertEqual(len(df), 4)  # 4行のデータがあるはず
            self.assertIn('name', df.columns)
            self.assertIn('x', df.columns)
            self.assertIn('wl', df.columns)
            self.assertIn('wr', df.columns)
        finally:
            # 出力ファイルを削除
            if os.path.exists(output_path):
                os.remove(output_path)

if __name__ == '__main__':
    unittest.main() 