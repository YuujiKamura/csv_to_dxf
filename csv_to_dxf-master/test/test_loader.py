import unittest
import pandas as pd
import loader
from unittest.mock import patch, MagicMock

class TestLoader(unittest.TestCase):
    
    @classmethod
    def setUpClass(cls):
        # テスト実行前にヘッドレスモードを有効にする
        loader.set_headless_mode(True)
    
    @classmethod
    def tearDownClass(cls):
        # テスト終了後にヘッドレスモードを無効に戻す
        loader.set_headless_mode(False)
    
    def setUp(self):
        # テスト用のサンプルデータを作成
        self.test_data = pd.DataFrame({
            'name': ['No.0', 'No.1', 'No.2'],
            'x': [0.0, 10.0, 20.0],
            'wl': [3.45, 3.50, 3.45],
            'wr': [3.55, 3.50, 3.55]
        })
    
    @patch('pandas.read_clipboard')
    def test_load_data_from_clipboard_success(self, mock_read_clipboard):
        # クリップボードからの読み込み成功ケース
        mock_read_clipboard.return_value = pd.DataFrame({
            0: ['No.0', 'No.1', 'No.2'],
            1: [0.0, 10.0, 20.0],
            2: [3.45, 3.50, 3.45],
            3: [3.55, 3.50, 3.55]
        })
        
        result = loader.load_data_from_clipboard()
        self.assertIsNotNone(result)
        self.assertEqual(result.shape, (3, 4))
        self.assertEqual(list(result.columns), ['name', 'x', 'wl', 'wr'])
    
    @patch('pandas.read_clipboard')
    @patch('loader.show_error_dialog')
    def test_load_data_from_clipboard_parse_error(self, mock_show_error, mock_read_clipboard):
        # パースエラーの場合のテスト
        mock_read_clipboard.side_effect = pd.errors.ParserError("テスト用エラー")
        
        result = loader.load_data_from_clipboard()
        self.assertIsNone(result)
        # エラーダイアログが表示されたことを確認
        mock_show_error.assert_called_once()
    
    @patch('pandas.read_clipboard')
    @patch('loader.show_error_dialog')
    def test_load_data_from_clipboard_exception(self, mock_show_error, mock_read_clipboard):
        # その他の例外が発生した場合のテスト
        mock_read_clipboard.side_effect = Exception("テスト用エラー")
        
        result = loader.load_data_from_clipboard()
        self.assertIsNone(result)
        # エラーダイアログが表示されたことを確認
        mock_show_error.assert_called_once()
    
    @patch('pandas.read_csv')
    def test_load_data_from_csv(self, mock_read_csv):
        # CSVからの読み込みテスト
        mock_read_csv.return_value = self.test_data
        
        result = loader.load_data_from_csv()
        self.assertIsNotNone(result)
        mock_read_csv.assert_called_once_with('data.csv')
    
    def test_validate_data_valid(self):
        # 有効なデータの検証テスト
        result = loader.validate_data(self.test_data)
        self.assertIsNotNone(result)
    
    @patch('loader.show_data_in_dialog')
    @patch('loader.show_error_dialog')
    def test_validate_data_invalid_columns(self, mock_show_error, mock_show_data):
        # 列数が無効なデータの検証テスト
        invalid_data = pd.DataFrame({
            'name': ['No.0', 'No.1'],
            'x': [0.0, 10.0],
            'wl': [3.45, 3.50]
        })
        
        result = loader.validate_data(invalid_data)
        self.assertIsNone(result)
        # エラーダイアログが表示されたことを確認
        mock_show_error.assert_called_once()
        mock_show_data.assert_called_once()

if __name__ == '__main__':
    unittest.main() 