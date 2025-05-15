import os
import unittest
import pandas as pd
import ezdxf
import traceback
import dxf_draw_tenkaiz
import glob
import argparse
import subprocess
import sys
import inspect
import sys
import os

# プロジェクトルートへのパスを追加
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..')))


# モジュールを条件付きでインポート
try:
    import src.ezdxf as ezdxf
except ImportError:
    try:
        import ezdxf
    except ImportError:
        print("警告: ezdxfモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.traceback as traceback
except ImportError:
    try:
        import traceback
    except ImportError:
        print("警告: tracebackモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.dxf_draw_tenkaiz as dxf_draw_tenkaiz
except ImportError:
    try:
        import dxf_draw_tenkaiz
    except ImportError:
        print("警告: dxf_draw_tenkaizモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.glob as glob
except ImportError:
    try:
        import glob
    except ImportError:
        print("警告: globモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.argparse as argparse
except ImportError:
    try:
        import argparse
    except ImportError:
        print("警告: argparseモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.subprocess as subprocess
except ImportError:
    try:
        import subprocess
    except ImportError:
        print("警告: subprocessモジュールが見つかりません。テストはスキップされます。")


# モジュールを条件付きでインポート
try:
    import src.inspect as inspect
except ImportError:
    try:
        import inspect
    except ImportError:
        print("警告: inspectモジュールが見つかりません。テストはスキップされます。")



class TestCSVtoDXF(unittest.TestCase):
    """区間CSVからDXFへの変換テスト"""
    
    def setUp(self):
        """テスト環境のセットアップ"""
        self.csv_file = '区間1.csv'
        self.output_dxf = '区間1.dxf'
        
        # テスト実行前に出力ファイルが存在する場合は削除
        if os.path.exists(self.output_dxf):
            os.remove(self.output_dxf)
    
    def tearDown(self):
        """テスト環境のクリーンアップ"""
        # テスト完了後、生成されたDXFファイルを削除（オプション）
        # if os.path.exists(self.output_dxf):
        #     os.remove(self.output_dxf)
        pass
    
    def test_convert_csv_to_dxf(self):
        """区間CSVからDXFファイルへの変換テスト"""
        # 変換パラメータ設定
        scale = 1000.0       # スケール倍率
        text_height = 350.0  # テキスト高さ

        # CSVファイルの存在確認
        self.assertTrue(os.path.exists(self.csv_file), f"CSVファイル {self.csv_file} が見つかりません")
        
        # CSVデータの読み込み
        data = pd.read_csv(self.csv_file)
        print(f"\n読み込んだCSVデータ ({self.csv_file}):")
        print(data)
        
        try:
            # draw_road_sections関数のシグネチャを確認
            print("\n関数シグネチャの確認:")
            sig = inspect.signature(dxf_draw_tenkaiz.draw_road_sections)
            print(f"draw_road_sections関数: {sig}")
            
            # DXF変換処理の実行
            dxf = ezdxf.new('R2010')  # 新しいDXFドキュメントの作成
            
            # モデル空間の取得
            msp = dxf.modelspace()
            
            # 測点、幅員データからDXF作図
            print("\nDXF変換開始...")
            print(f"スケール: {scale}倍, テキスト高さ: {text_height}")
            dxf_draw_tenkaiz.draw_road_sections(msp, data, scale=scale, text_height=text_height)
            
            # DXFファイルの保存
            dxf.saveas(self.output_dxf)
            
            # 変換結果の検証
            self.assertTrue(os.path.exists(self.output_dxf), "DXFファイルが生成されていません")
            
            # 生成されたDXFファイルのサイズを確認
            file_size = os.path.getsize(self.output_dxf)
            print(f"\n生成されたDXFファイル: {self.output_dxf} (サイズ: {file_size} バイト)")
            self.assertGreater(file_size, 0, "生成されたDXFファイルが空です")
            
            # DXFファイルの内容を読み込んで検証
            verify_dxf = ezdxf.readfile(self.output_dxf)
            msp_verify = verify_dxf.modelspace()
            
            # エンティティカウント（直線、円弧など）
            entities_count = len(list(msp_verify))
            print(f"DXFエンティティ数: {entities_count}")
            self.assertGreater(entities_count, 0, "DXFにエンティティが含まれていません")
            
            # 図面の範囲を取得
            all_entity_points = []
            for entity in msp_verify:
                if hasattr(entity, 'dxf') and hasattr(entity.dxf, 'start'):
                    all_entity_points.append((entity.dxf.start[0], entity.dxf.start[1]))
                if hasattr(entity, 'dxf') and hasattr(entity.dxf, 'end'):
                    all_entity_points.append((entity.dxf.end[0], entity.dxf.end[1]))
            
            if all_entity_points:
                min_x = min(p[0] for p in all_entity_points)
                max_x = max(p[0] for p in all_entity_points)
                min_y = min(p[1] for p in all_entity_points)
                max_y = max(p[1] for p in all_entity_points)
                
                width = max_x - min_x
                height = max_y - min_y
                
                print(f"\n生成された図面の範囲:")
                print(f"X: {min_x} 〜 {max_x}, Y: {min_y} 〜 {max_y}")
                print(f"幅: {width}, 高さ: {height}")
                
                self.assertTrue(width > 0, "図面の幅が0以下です")
                self.assertTrue(height > 0, "図面の高さが0以下です")
            
            print("\nDXF変換テスト成功")
        
        except Exception as e:
            print(f"\nDXF変換中にエラーが発生しました: {e}")
            print("\n詳細なエラー情報:")
            traceback.print_exc()
            self.fail(f"DXF変換中にエラーが発生しました: {str(e)}")


def manual_convert_csv_to_dxf(csv_file, output_dxf, scale=1000.0, text_height=350.0):
    """
    CSVからDXFへの変換を行う手動関数
    
    Args:
        csv_file: 入力CSVファイルのパス
        output_dxf: 出力DXFファイルのパス
        scale: 図形のスケール倍率
        text_height: テキストの高さ
        
    Returns:
        bool: 変換が成功したかどうか
    """
    try:
        # CSVデータの読み込み
        data = pd.read_csv(csv_file)
        print(f"\n読み込んだCSVデータ ({csv_file}):")
        print(data)
        
        # draw_road_sections関数のシグネチャを確認
        print("\n関数シグネチャの確認:")
        sig = inspect.signature(dxf_draw_tenkaiz.draw_road_sections)
        print(f"draw_road_sections関数: {sig}")
        
        # DXF変換処理の実行
        dxf = ezdxf.new('R2010')
        msp = dxf.modelspace()
        
        # 測点、幅員データからDXF作図
        print("\nDXF変換開始...")
        print(f"スケール: {scale}倍, テキスト高さ: {text_height}")
        dxf_draw_tenkaiz.draw_road_sections(msp, data, scale=scale, text_height=text_height)
        
        # DXFファイルの保存
        dxf.saveas(output_dxf)
        
        # 変換結果の確認
        if os.path.exists(output_dxf):
            file_size = os.path.getsize(output_dxf)
            print(f"\n生成されたDXFファイル: {output_dxf} (サイズ: {file_size} バイト)")
            
            # DXFファイルの内容を読み込んで検証
            verify_dxf = ezdxf.readfile(output_dxf)
            msp_verify = verify_dxf.modelspace()
            
            # エンティティカウント（直線、円弧など）
            entities_count = len(list(msp_verify))
            print(f"DXFエンティティ数: {entities_count}")
            
            print("\nDXF変換成功")
            return True
        else:
            print("\nDXF変換失敗: ファイルが生成されませんでした")
            return False
    
    except Exception as e:
        print(f"\nDXF変換中にエラーが発生しました: {e}")
        print("\n詳細なエラー情報:")
        traceback.print_exc()
        return False


# coodinate_dimensions関数のインターフェースを確認する関数
def inspect_coodinate_dimensions():
    """coodinate_dimensions関数のインターフェースを検査"""
    print("\ncoodinate_dimensions関数の詳細:")
    sig = inspect.signature(dxf_draw_tenkaiz.coodinate_dimensions)
    print(f"関数シグネチャ: {sig}")
    print(f"ドキュメント: {dxf_draw_tenkaiz.coodinate_dimensions.__doc__}")
    
    # 関数の引数名とデフォルト値を表示
    for param_name, param in sig.parameters.items():
        print(f"  引数: {param_name}, デフォルト値: {param.default if param.default is not param.empty else 'なし'}")


# すべての区間ファイルを一括変換する関数
def convert_all_sections(scale=1000.0, text_height=350.0):
    """すべての区間CSVファイルをDXFに変換する"""
    csv_files = glob.glob('区間*.csv')
    
    if not csv_files:
        print("区間CSVファイルが見つかりません")
        return False
    
    success_count = 0
    fail_count = 0
    
    for csv_file in csv_files:
        dxf_file = csv_file.replace('.csv', '.dxf')
        print(f"\n=== {csv_file} を {dxf_file} に変換 ===")
        
        if manual_convert_csv_to_dxf(csv_file, dxf_file, scale, text_height):
            success_count += 1
        else:
            fail_count += 1
    
    print(f"\n変換結果サマリー:")
    print(f"変換成功: {success_count} ファイル")
    print(f"変換失敗: {fail_count} ファイル")
    
    return success_count > 0


# モジュール内の関数を確認するデバッグ関数
def inspect_module_functions():
    """dxf_draw_tenkaizモジュール内の関数定義を確認する"""
    print("\ndxf_draw_tenkaizモジュール内の関数定義:")
    
    # 関数のリストを取得
    functions = [name for name, obj in inspect.getmembers(dxf_draw_tenkaiz) 
                 if inspect.isfunction(obj)]
    
    print(f"利用可能な関数: {functions}")
    
    # draw_road_sections関数の詳細を表示
    print("\ndraw_road_sections関数の詳細:")
    sig = inspect.signature(dxf_draw_tenkaiz.draw_road_sections)
    print(f"関数シグネチャ: {sig}")
    print(f"ドキュメント: {dxf_draw_tenkaiz.draw_road_sections.__doc__}")


def open_dxf_viewer(dxf_file_path):
    """DXFビューワでDXFファイルを開く
    
    Args:
        dxf_file_path: 表示するDXFファイルのパス
        
    Returns:
        bool: ビューワの起動が成功したかどうか
    """
    try:
        # DXFビューワのパスを取得
        viewer_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "dxfviewer", "dxf_viewer.py"))
        
        if not os.path.exists(viewer_path):
            print(f"エラー: DXFビューワが見つかりません: {viewer_path}")
            return False
            
        # DXFファイルのフルパス取得
        dxf_file_full_path = os.path.abspath(dxf_file_path)
        
        if not os.path.exists(dxf_file_full_path):
            print(f"エラー: DXFファイルが見つかりません: {dxf_file_full_path}")
            return False
            
        # DXFビューワを起動
        print(f"\nDXFビューワで開きます: {dxf_file_full_path}")
        
        # Pythonの解釈子を取得
        python_exe = sys.executable
        
        # サブプロセスとしてビューワを起動
        subprocess.Popen([python_exe, viewer_path, dxf_file_full_path], 
                         stdout=subprocess.DEVNULL,  # 標準出力を表示しない
                         stderr=subprocess.DEVNULL,  # 標準エラー出力を表示しない
                         creationflags=subprocess.CREATE_NEW_CONSOLE)  # 新しいコンソールで起動
        
        print(f"DXFビューワを起動しました")
        return True
        
    except Exception as e:
        print(f"DXFビューワの起動中にエラーが発生しました: {e}")
        traceback.print_exc()
        return False


if __name__ == "__main__":
    # コマンドライン引数の処理
    parser = argparse.ArgumentParser(description='CSVファイルをDXFに変換するツール')
    parser.add_argument('--scale', type=float, default=1000.0, help='図形のスケール倍率')
    parser.add_argument('--text-height', type=float, default=350.0, help='幅員表示用テキストの高さ')
    parser.add_argument('--all', action='store_true', help='すべての区間ファイルを変換')
    parser.add_argument('--file', type=str, help='変換する特定のCSVファイル')
    parser.add_argument('--open-viewer', action='store_true', help='変換後にDXFビューワで開く')
    parser.add_argument('--debug', action='store_true', help='デバッグ情報を表示')
    
    args = parser.parse_args()
    scale = args.scale
    text_height = args.text_height
    open_viewer = args.open_viewer
    debug = args.debug
    
    print(f"CSV to DXF 変換ツール")
    print(f"スケール: {scale}倍, テキスト高さ: {text_height}\n")
    
    # デバッグモードの場合、モジュール内の関数を確認
    if debug:
        inspect_module_functions()
        inspect_coodinate_dimensions()
    
    dxf_file_path = None  # 生成されたDXFファイルのパスを格納
    
    if args.all:
        # すべての区間ファイルを変換
        success = convert_all_sections(scale, text_height)
        
        # 最初のDXFファイルをビューワで開く（複数ある場合）
        if success and open_viewer:
            dxf_files = glob.glob('区間*.dxf')
            if dxf_files:
                dxf_file_path = dxf_files[0]
    
    elif args.file:
        # 指定されたファイルを変換
        if not args.file.endswith('.csv'):
            args.file += '.csv'
        
        output_dxf = args.file.replace('.csv', '.dxf')
        
        if os.path.exists(args.file):
            print(f"CSVファイル {args.file} からDXFへの変換を開始します")
            success = manual_convert_csv_to_dxf(args.file, output_dxf, scale, text_height)
            
            if success:
                print(f"変換完了: {output_dxf} が生成されました")
                dxf_file_path = output_dxf
            else:
                print("変換に失敗しました")
        else:
            print(f"CSVファイル {args.file} が見つかりません")
    
    else:
        # デフォルトの処理（区間1.csvの変換）
        csv_file = '区間1.csv'
        output_dxf = '区間1.dxf'
        
        if os.path.exists(csv_file):
            print(f"CSVファイル {csv_file} からDXFへの変換を開始します")
            success = manual_convert_csv_to_dxf(csv_file, output_dxf, scale, text_height)
            
            if success:
                print(f"変換完了: {output_dxf} が生成されました")
                dxf_file_path = output_dxf
            else:
                print("変換に失敗しました")
        else:
            print(f"CSVファイル {csv_file} が見つかりません")
    
    # 変換に成功し、ファイルが生成され、--open-viewerオプションが指定されている場合はDXFビューワで開く
    if dxf_file_path and open_viewer:
        open_dxf_viewer(dxf_file_path) 