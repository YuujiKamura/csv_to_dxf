"""
テスト環境設定ファイル
テストの実行設定やフィクスチャを定義します。
"""

import os
import sys
import pytest

# プロジェクトルートへのパスを取得
PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

# プロジェクトルートをPythonパスに追加
sys.path.insert(0, PROJECT_ROOT)


def pytest_configure(config):
    """
    pytestの設定を行う関数
    テスト実行前に呼び出される
    """
    print(f"テスト環境の設定: プロジェクトルート = {PROJECT_ROOT}")
    

@pytest.fixture(scope="session")
def test_data_dir():
    """テストデータディレクトリのパスを返すフィクスチャ"""
    data_dir = os.path.join(PROJECT_ROOT, "data")
    
    # データディレクトリが存在することを確認
    if not os.path.exists(data_dir):
        # テスト用のデータディレクトリがない場合は作成
        os.makedirs(data_dir, exist_ok=True)
        print(f"警告: テスト用データディレクトリ {data_dir} を作成しました")
    
    return data_dir


@pytest.fixture(scope="session")
def temp_dir(tmpdir_factory):
    """テスト用の一時ディレクトリを提供するフィクスチャ"""
    return tmpdir_factory.mktemp("test_output")


@pytest.fixture(scope="session")
def sample_csv_file(test_data_dir):
    """サンプルCSVファイルのパスを返すフィクスチャ"""
    # サンプルCSVファイルのパス
    sample_csv = os.path.join(test_data_dir, "sample.csv")
    
    # CSVファイルが存在しない場合、テスト用のサンプルを作成
    if not os.path.exists(sample_csv):
        with open(sample_csv, 'w', encoding='utf-8') as f:
            f.write(""",,,,
区間1,台形計算,,,
測点名,単延長L,幅員W,平均幅員Wa,面積m2
No.0,0,0.8,,
,1.15,0.63,0.715,0.82

区間2,台形計算,,,
測点名,単延長L,幅員W,平均幅員Wa,面積m2
No.5,0,0.5,,
,2.0,0.5,0.5,1.0
""")
        print(f"警告: サンプルCSVファイル {sample_csv} を作成しました")
    
    return sample_csv


@pytest.fixture(scope="session")
def setup_test_paths():
    """テストディレクトリとファイルパスを設定するフィクスチャ"""
    # 新しいテストディレクトリ構造の確認と作成
    for test_dir in ["unit", "integration", "e2e"]:
        dir_path = os.path.join(os.path.dirname(__file__), test_dir)
        if not os.path.exists(dir_path):
            os.makedirs(dir_path, exist_ok=True)
            # 初期化ファイルを作成
            with open(os.path.join(dir_path, "__init__.py"), "w") as f:
                f.write(f"# {test_dir.capitalize()} tests\n")
    
    print("テストディレクトリ構造を確認しました")
    
    return {
        "unit_dir": os.path.join(os.path.dirname(__file__), "unit"),
        "integration_dir": os.path.join(os.path.dirname(__file__), "integration"),
        "e2e_dir": os.path.join(os.path.dirname(__file__), "e2e")
    } 