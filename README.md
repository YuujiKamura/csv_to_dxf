# CSV to DXF 変換ツール

CSVファイルから道路断面図のDXFファイルを生成するツールです。

**[🌐 Web DXF Viewer](https://yuujikamura.github.io/csv_to_dxf/)** - ブラウザでDXFファイルを閲覧できます

## 機能

### デスクトップアプリケーション (Python/PyQt6)
- CSVファイルから区間データを抽出・処理
- 道路断面図のDXFファイルを生成
- 複数区間の一括処理
- リアルタイムプレビュー

### Web DXF Viewer (Rust/WebAssembly)
- ブラウザ上でDXFファイルを表示
- ドラッグ＆ドロップでファイル読み込み
- パン・ズーム操作
- ライト/ダークテーマ切り替え

## インストール

### Python環境から実行する場合

```bash
# リポジトリをクローン
git clone https://github.com/YuujiKamura/csv_to_dxf.git
cd csv_to_dxf

# 依存関係をインストール
pip install pandas ezdxf PyQt6
```

### Windows環境で直接実行する場合

Python環境のインストールは不要です：
1. [Releases](https://github.com/YuujiKamura/csv_to_dxf/releases)から最新版をダウンロード
2. `csv_to_dxf.exe` を実行

## 使い方

### デスクトップアプリケーション

```bash
python main.py
```

1. 「参照...」ボタンでCSVファイルを選択
2. 「区間」プルダウンから処理したい区間を選択
3. 「プレビュー」ボタンでデータを確認
4. 「DXFに変換」ボタンでDXFファイルを生成

### Web DXF Viewer

https://yuujikamura.github.io/csv_to_dxf/ にアクセスして：
- 「Open DXF」ボタンでファイルを選択、または
- DXFファイルをドラッグ＆ドロップ

## 入力CSVファイル形式

| 項目 | 説明 | 例 |
|------|------|-----|
| 区間名 | 区間を識別する名前 | 区間1 |
| 測点名 | 測点の名前 | No.1+10 |
| 単延長 | 測点間の距離 | 20.0 |
| 幅員 | 道路の幅員 | 5.5 |

## オプション

- **単延長の自動変換**: 単延長データを累積距離（追加延長）に変換
- **測点名の正規化**: 測点名を連番形式に統一
- **左右の幅員入れ替え**: 必要に応じて左右の幅員を入れ替え

## プロジェクト構成

```
csv_to_dxf/
├── main.py                 # アプリケーションエントリーポイント
├── gui/                    # PyQt6 GUI
│   ├── app_controller.py   # アプリケーションコントローラー
│   ├── core/               # CSV処理、DXF生成
│   └── ui/                 # UIコンポーネント
├── src/                    # ユーティリティ
│   ├── dxf_draw_tenkaiz.py # DXF描画処理
│   ├── processing.py       # データ処理
│   └── station_name_utils.py # 測点名処理
├── web/                    # Rust/WebAssembly DXFビューア
│   ├── src/                # Rustソースコード
│   └── www/                # 静的ファイル (GitHub Pages)
├── tests/                  # テストスイート
└── data/                   # サンプルデータ
```

## 開発

### テストの実行

```bash
# 全テスト実行
pytest tests/

# 特定のテスト実行
python tests/test_save_dxf.py
```

### Webビューアのビルド

```bash
cd web
wasm-pack build --target web --out-dir www/pkg
```

## 技術スタック

- **Python**: pandas, ezdxf, PyQt6
- **Rust + WebAssembly**: dxf crate, wasm-bindgen
- **CI/CD**: GitHub Actions (自動ビルド・GitHub Pagesデプロイ)

## ライセンス

GNU General Public License v3.0

このアプリケーションはPyQt6を使用しており、GPL v3ライセンスの下で公開されています。
派生物を配布する場合は、同じライセンス条件でソースコードを公開する必要があります。
