# CSV to DXF 変換ツール

![アプリケーションプレビュー](screenshots/app_preview.png)

CSVファイルから道路断面図のDXFファイルを自動生成するツールです。測量データや道路設計データをCAD図面に簡単に変換できます。

## 主な機能

- **CSV読み込み**: 道路測量データを含むCSVファイルを読み込み
- **区間抽出**: ファイル内の複数区間から対象を選択・抽出
- **DXF生成**: 道路断面図をCAD互換のDXFフォーマットで出力
- **一括処理**: 複数区間の一括処理に対応
- **GUI/CLI両対応**: PyQt6によるGUIとコマンドラインの両方で利用可能
- **Windows EXE**: Python環境なしで実行可能なスタンドアロンアプリケーション

## 処理オプション

| オプション | 説明 |
|-----------|------|
| 単延長の自動変換 | 単延長データを累積距離（追加延長）に変換 |
| 測点名の正規化 | 測点名を連番形式（No.0, No.1, ...）に統一 |
| 左右の幅員入れ替え | 特定区間用に左右の幅員を入れ替え |
| 幅員割り当て | 左幅員/右幅員の選択 |

## インストール

### Python環境から実行する場合

```bash
# リポジトリをクローン
git clone https://github.com/YuujiKamura/csv_to_dxf.git
cd csv_to_dxf

# 必要なライブラリをインストール
pip install pandas ezdxf PyQt6 requests
```

### Windows環境で直接実行する場合

Python環境のインストールは不要です：

1. GitHubリポジトリからクローンまたはダウンロード
2. `dist/csv_to_dxf/csv_to_dxf.exe` を実行

## 使い方

### GUIアプリケーション

1. アプリケーションを起動

```bash
# Python環境から
python main.py

# またはWindows環境では
dist/csv_to_dxf/csv_to_dxf.exe
```

2. 「参照...」ボタンをクリックしてCSVファイルを選択
3. 「区間」プルダウンから処理したい区間を選択
4. 「プレビュー」ボタンをクリックしてデータをプレビュー
5. 「DXFに変換」ボタンをクリックしてDXFファイルを生成
   - ファイル保存ダイアログには、選択した区間名がデフォルトのファイル名として自動設定されます

### コマンドライン操作

```bash
# 道路断面データをクリップボードからDXFに変換
python src/table_to_dxf.py
```

## 入力データ形式

### CSVファイル形式

入力CSVファイルには以下の項目が必要です：

| 列名 | 説明 | 例 |
|-----|------|-----|
| 区間名 | 区間を識別する名前 | 区間1, 区間2 |
| 測点名 | 測点の名前 | No.0, No.1+10 |
| 単延長 | 測点間の距離（m） | 10, 20.5 |
| 幅員 | 道路の幅員（m） | 3.50, 4.20 |

### クリップボード入力形式

4列のデータ形式：

| 測点名 | 距離(m) | 左幅員(m) | 右幅員(m) |
|--------|---------|-----------|-----------|
| No.0   | 0       | 3.50      | 3.35      |
| No.1   | 10      | 3.40      | 3.35      |

**注意:**
- 片側幅員のみの場合、もう片方は0を入力
- 小数値は正確に処理されます

## プロジェクト構成

```
csv_to_dxf/
├── main.py                    # GUIアプリのエントリーポイント
├── app_config.json            # アプリケーション設定
│
├── gui/                       # GUIアプリケーション
│   ├── app_controller.py      # アプリコントローラ
│   ├── core/                  # コア機能モジュール
│   │   ├── csv_processor.py   # CSV処理
│   │   ├── dxf_generator.py   # DXF生成
│   │   └── config_manager.py  # 設定管理
│   └── ui/                    # UIコンポーネント
│       └── main_window.py     # メインウィンドウ
│
├── src/                       # コア処理モジュール
│   ├── processing.py          # CSV抽出・変換
│   ├── exporter.py            # ファイルエクスポート
│   ├── dxf_draw_tenkaiz.py    # DXF描画エンジン
│   ├── station_name_utils.py  # 測点処理ユーティリティ
│   └── table_to_dxf.py        # クリップボード→DXF変換
│
├── modules/                   # 専用機能モジュール
│   └── board_collector/       # ボード写真収集機能
│
├── tests/                     # テストスイート
│   ├── e2e/                   # E2Eテスト
│   ├── integration/           # 統合テスト
│   └── unit/                  # ユニットテスト
│
├── data/                      # サンプルデータ
├── dist/csv_to_dxf/           # ビルド済みWindowsアプリ
└── generate_dxf_kouyama.py    # Google Sheets連携スクリプト
```

## 特殊機能

### Google Sheets連携

`generate_dxf_kouyama.py` を使用して、Google Sheetsから直接データを取得してDXFを生成できます：

```bash
python generate_dxf_kouyama.py
```

### ボード写真収集

`modules/board_collector/` モジュールで工事写真のボードデータを収集できます。

### スクリーンショット機能

メニューから「スクリーンショットを保存」で、プレビュー画面を `screenshots/` ディレクトリに自動保存できます。

## テスト

### 自動テスト

```bash
# 全テストを実行
python -m pytest tests/

# 個別テストを実行
python -m pytest tests/e2e/test_csv_to_dxf.py
python -m pytest tests/integration/test_app_controller.py
```

### 手動テスト

```bash
# テストスクリプトを実行
python tests/test_save_dxf.py
```

## 出力ファイル

生成されたDXFファイルは以下のソフトウェアで開くことができます：

- AutoCAD
- Jw_cad
- LibreCAD
- その他DXF対応CADソフトウェア

## 動作環境

- **Python**: 3.6以上
- **OS**: Windows, Linux, macOS

### 依存ライブラリ

| ライブラリ | 用途 |
|-----------|------|
| pandas | データ処理 |
| ezdxf | DXF生成 |
| PyQt6 | GUI（GPLライセンス） |
| requests | HTTP通信（Google Sheets取得用） |

## トラブルシューティング

### 「クリップボードからデータを読み込めませんでした」

- データが正しくクリップボードにコピーされているか確認
- データ形式が4列になっているか確認
- スプレッドシートで正確にセル範囲を選択してからコピー

### 「DXFファイルが作成されない」

- コンソール出力でエラーメッセージを確認
- 入力データ形式を確認
- 既存のDXFファイルが開いていないか確認（上書きできない場合があります）

## 更新履歴

### 2025-04-30: Windows実行ファイル（EXE）の追加

- Windows用の実行ファイル（EXE）を追加
- Python環境なしで実行可能なスタンドアロンアプリケーション

### 2025-04-29: DXF保存機能の修正

- `CSVProcessor.save_dxf()` メソッドのパラメータ不一致を修正
- `get_section()` メソッドでのDataFrame真偽値評価エラーを修正
- エラーと成功時のログ出力を強化

## ライセンス

このプロジェクトはGNU General Public License (GPL) バージョン3の下で公開されています。

**注意**: このアプリケーションはPyQt6を使用しており、PyQt6はGPLライセンスの下で提供されています。そのため、このアプリケーション全体（ソースコードおよびバイナリ）もGPLライセンスの制約を受けます。GPLはコピーレフトライセンスであり、このソフトウェアの派生物を配布する場合は、同じライセンス条件（GPL）の下でソースコードを公開する必要があります。

## 著作権

© 2025 Yuuji Softworks
