# Google Sheets PDF エクスポートモジュール

Google スプレッドシートの特定シートをPDF化し、オプションでプレビュー表示、Google Driveへのアップロードを行う汎用モジュールです。

## 概要

このモジュールは、11/3の工事週報PDF生成スクリプトから得られた知見を汎用化したものです。以下の機能を提供します:

### 主要機能

1. **柔軟なシート選択**
   - シート名による完全一致、前方一致、部分一致
   - シートIDによる直接指定
   - カスタムフィルタによる条件指定
   - 複数マッチ時の最新選択（番号による）

2. **データ範囲の自動検出**
   - セルの値だけでなく、罫線情報も考慮
   - 実際に使用されている範囲のみをPDF化

3. **詳細なPDFエクスポート設定**
   - ページ向き（縦/横）
   - 用紙サイズ（A4, A3, LETTER, LEGAL）
   - フィット設定（幅、高さ）
   - マージン設定
   - グリッド線、シート名、ページ番号の表示制御

4. **GUIプレビュー機能**
   - PyMuPDFとTkinterによるPDFプレビュー
   - ユーザーによる承認・拒否の取得

5. **Google Driveアップロード**
   - 指定フォルダへの自動アップロード
   - 同名ファイルの置き換え対応

## インストール

必要なパッケージをインストールしてください:

```bash
pip install google-auth google-auth-oauthlib google-auth-httplib2 google-api-python-client requests PyMuPDF Pillow
```

## 基本的な使い方

### 1. 認証情報の準備

Google API認証情報を用意します:

```python
from google.oauth2.credentials import Credentials

TOKEN_FILE = "path/to/gmail_token.json"
SCOPES = [
    'https://www.googleapis.com/auth/drive',
    'https://www.googleapis.com/auth/spreadsheets'
]

creds = Credentials.from_authorized_user_file(TOKEN_FILE, SCOPES)
```

### 2. エクスポーターの作成

```python
from sheets_pdf_exporter import SheetsPDFExporter

exporter = SheetsPDFExporter(creds)
```

### 3. シンプルなPDFエクスポート

```python
from sheets_pdf_exporter import SheetSelector

spreadsheet_id = '1RqW57aq7-QnoH37rg0TxivyiejJ6uwYBnW8iQGGh7Ac'
sheet_selector = SheetSelector(sheet_name='Sheet1')

# PDFとして保存
exporter.export_and_save(spreadsheet_id, sheet_selector, 'output.pdf')
```

## 高度な使い方

### 最新の週報シートを自動選択

```python
from sheets_pdf_exporter import SheetSelector

# 'shuuho_' で始まるシートのうち、番号が最大のものを選択
sheet_selector = SheetSelector(
    sheet_name_prefix='shuuho_',
    select_latest=True
)

exporter.export_and_save(spreadsheet_id, sheet_selector, 'weekly_report.pdf')
```

### カスタムPDF設定

```python
from sheets_pdf_exporter import PDFExportConfig, PageOrientation, PaperSize

config = PDFExportConfig(
    orientation=PageOrientation.LANDSCAPE,  # 横向き
    paper_size=PaperSize.A3,  # A3サイズ
    show_gridlines=True,  # グリッド線を表示
    top_margin=0.5,  # 上マージン（インチ）
    bottom_margin=0.5,
    left_margin=0.5,
    right_margin=0.5,
)

exporter.export_and_save(
    spreadsheet_id,
    sheet_selector,
    'output_a3_landscape.pdf',
    config=config
)
```

### プレビュー表示してからアップロード

```python
from pdf_preview_gui import show_pdf_preview

# 一時ファイルにエクスポート
temp_pdf = exporter.export_to_temp_file(spreadsheet_id, sheet_selector)

# プレビューを表示
approved = show_pdf_preview(temp_pdf, title="工事週報プレビュー")

if approved:
    # PDFデータを読み込み
    with open(temp_pdf, 'rb') as f:
        pdf_data = f.read()

    # Driveにアップロード
    DRIVE_FOLDER_ID = '18myb0cJ71imEuGyZnz6eAA8a7YLMri--'
    file_id = exporter.upload_to_drive(
        pdf_data,
        '工事週報.pdf',
        folder_id=DRIVE_FOLDER_ID,
        replace_existing=True
    )
    print(f"アップロード完了: {file_id}")

# 一時ファイルを削除
os.remove(temp_pdf)
```

### 直接アップロード（プレビューなし）

```python
# PDFを生成してDriveに直接アップロード
file_id = exporter.export_and_upload(
    spreadsheet_id,
    sheet_selector,
    'weekly_report.pdf',
    folder_id=DRIVE_FOLDER_ID,
    replace_existing=True
)
```

### カスタムフィルタでシート選択

```python
# 非表示でないシートのみを選択
def not_hidden_filter(sheet_props):
    return not sheet_props.get('hidden', False)

sheet_selector = SheetSelector(
    sheet_name_prefix='shuuho_',
    sheet_filter=not_hidden_filter
)
```

## API リファレンス

### SheetsPDFExporter クラス

#### メソッド

- `__init__(credentials: Credentials)`: 初期化
- `get_spreadsheet_info(spreadsheet_id: str) -> Dict`: スプレッドシート情報を取得
- `find_sheets(spreadsheet_id: str, selector: SheetSelector) -> List[Dict]`: 条件に合致するシートを検索
- `export_sheet_to_pdf(...) -> bytes`: シートをPDF化してバイトデータを返す
- `export_and_save(...) -> str`: シートをPDF化してファイルに保存
- `export_to_temp_file(...) -> str`: シートをPDF化して一時ファイルに保存
- `upload_to_drive(...) -> str`: PDFをGoogle Driveにアップロード
- `export_and_upload(...) -> str`: PDF化とアップロードを一度に実行

### SheetSelector クラス

シート選択条件を指定するデータクラス

#### 属性

- `sheet_name: Optional[str]`: シート名（完全一致）
- `sheet_name_prefix: Optional[str]`: シート名の前方一致
- `sheet_name_contains: Optional[str]`: シート名の部分一致
- `sheet_id: Optional[int]`: シートID（直接指定）
- `select_latest: bool`: 複数マッチ時に最新（番号が最大）を選択するか
- `sheet_filter: Optional[Callable]`: カスタムフィルタ関数

### PDFExportConfig クラス

PDFエクスポート設定を指定するデータクラス

#### 属性

- `orientation: PageOrientation`: ページ向き（PORTRAIT=縦, LANDSCAPE=横）
- `paper_size: PaperSize`: 用紙サイズ（A4, A3, LETTER, LEGAL）
- `fit_to_width: bool`: 幅に合わせる（デフォルト: True）
- `fit_to_height: bool`: 高さに合わせる（デフォルト: True）
- `scale: int`: スケール（デフォルト: 4）
- `show_gridlines: bool`: グリッド線を表示（デフォルト: False）
- `show_sheet_names: bool`: シート名を表示（デフォルト: False）
- `show_page_numbers: bool`: ページ番号を表示（デフォルト: False）
- `top_margin: float`: 上マージン（インチ、デフォルト: 0.39 ≈ 1cm）
- `bottom_margin: float`: 下マージン（インチ）
- `left_margin: float`: 左マージン（インチ）
- `right_margin: float`: 右マージン（インチ）
- `horizontal_alignment: str`: 水平配置（"CENTER", "LEFT", "RIGHT"）
- `vertical_alignment: str`: 垂直配置（"MIDDLE", "TOP", "BOTTOM"）

### PDFPreviewGUI クラス

PDFプレビューをGUIで表示

#### メソッド

- `__init__(pdf_path: str, ...)`: 初期化
- `show() -> bool`: プレビューを表示して承認を取得
- `show_pdf_and_get_approval(...) -> bool`: 静的メソッド、便利関数

#### 関数インターフェース

```python
from pdf_preview_gui import show_pdf_preview

approved = show_pdf_preview(pdf_path, title="プレビュー")
```

## 使用例

詳細な使用例は `sheets_pdf_example.py` を参照してください:

```bash
# 使用例を選択して実行
python src/sheets_pdf_example.py

# 特定の例を実行
python src/sheets_pdf_example.py 4  # 例4を実行

# すべての例を実行
python src/sheets_pdf_example.py all
```

## エラーハンドリング

```python
from sheets_pdf_exporter import SheetsAPIError
from pdf_preview_gui import PDFPreviewError

try:
    exporter.export_and_save(spreadsheet_id, sheet_selector, output_path)
except SheetsAPIError as e:
    print(f"エラー: {e}")
```

## 元のコードとの違い

11/3の工事週報スクリプトと比較した改善点:

1. **モジュール化**: 機能を再利用可能なクラスとして分離
2. **柔軟性**: 様々なシート選択方法に対応
3. **設定可能性**: PDFエクスポート設定を詳細にカスタマイズ可能
4. **エラーハンドリング**: 専用の例外クラスとエラーメッセージ
5. **ドキュメント**: 型ヒントとdocstringによる詳細なドキュメント
6. **使いやすさ**: 複数の便利メソッドを提供（ワンステップで実行可能）

## 今後の拡張案

- [ ] 複数シートを一つのPDFにまとめる機能
- [ ] PDFメタデータ（タイトル、作成者など）の設定
- [ ] ウォーターマーク追加機能
- [ ] バッチ処理（複数スプレッドシートを一度に処理）
- [ ] プレビューのズーム・ページナビゲーション機能強化
- [ ] PDF圧縮オプション

## ライセンス

プロジェクトのライセンスに従います。

## 参考

- 元のスクリプト: `C:/Users/yuuji/Sanyuu2Kouku/cursor_tools/summarygenerator/export_weekly_pdf_with_preview.py`
- Google Sheets API: https://developers.google.com/sheets/api
- Google Drive API: https://developers.google.com/drive/api
