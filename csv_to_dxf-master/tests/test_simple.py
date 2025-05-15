import os
import sys
import unittest

sys.path.append(os.path.join(os.path.dirname(__file__), '..'))
from gui.app_controller import AppController

class SimpleTest(unittest.TestCase):
    """GUIに依存しないシンプルなテスト"""
    
    def setUp(self):
        self.app_controller = AppController()
    
    def test_load_csv_file(self):
        """CSVファイルの読み込みテスト"""
        csv_path = os.path.abspath(os.path.join('data', '区間1.csv'))
        result = self.app_controller.load_file(csv_path)
        self.assertTrue(result, "ファイルの読み込みに失敗しました")
        
        # 区間が取得できることを確認
        sections = self.app_controller.get_available_sections()
        self.assertGreater(len(sections), 0, "区間が見つかりませんでした")
        self.assertIn("区間1", sections, "区間1が見つかりませんでした")
    
    def test_process_section_with_default_options(self):
        """基本的なデータ処理テスト"""
        csv_path = os.path.abspath(os.path.join('data', '区間1.csv'))
        self.app_controller.load_file(csv_path)
        
        # デフォルトオプションでデータ処理
        processed_data = self.app_controller.process_section("区間1")
        
        self.assertIsNotNone(processed_data, "データ処理に失敗しました")
        self.assertGreater(len(processed_data), 0, "処理結果が空です")
        
        # データの基本構造を確認
        self.assertIn('name', processed_data.columns, "name列がありません")
        self.assertIn('x', processed_data.columns, "x列がありません")
        self.assertIn('wl', processed_data.columns, "wl列がありません")
        self.assertIn('wr', processed_data.columns, "wr列がありません")

    def test_process_section_with_options(self):
        """オプション付きのデータ処理テスト"""
        csv_path = os.path.abspath(os.path.join('data', '区間1.csv'))
        self.app_controller.load_file(csv_path)
        
        # 様々なオプションでデータ処理
        processed_data = self.app_controller.process_section(
            "区間1", 
            auto_convert=True,
            normalize_station_names=False,
            swap_sides=True
        )
        
        self.assertIsNotNone(processed_data, "データ処理に失敗しました")
        self.assertGreater(len(processed_data), 0, "処理結果が空です")
        
        # 基本的なデータ構造の確認
        self.assertIn('name', processed_data.columns, "name列がありません")
        self.assertIn('x', processed_data.columns, "x列がありません")
        self.assertIn('wl', processed_data.columns, "wl列がありません")
        self.assertIn('wr', processed_data.columns, "wr列がありません")
        
        # swap_sidesオプションのテスト（詳細データに基づいて適宜修正）

if __name__ == '__main__':
    unittest.main() 