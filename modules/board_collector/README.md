# Board Photo Collector（ボード写真収集モジュール）

YOLOモデルを使用してフォルダから出来型ボード写真を自動収集する汎用モジュール。

## 特徴

- ✅ YOLOによる自動ボード検出
- ✅ ボード面積比率による客観的な分類（大・中・小・微小）
- ✅ MD5ハッシュによる重複除去
- ✅ 再帰的なフォルダ探索
- ✅ 柔軟なフィルタリング機能
- ✅ JSON形式での結果出力

## インストール

依存関係:
- Python 3.8+
- PIL (Pillow)
- summarygeneratorプロジェクト（YOLOPredictor）

## 使い方

### コマンドライン

```bash
# 基本的な使い方（10月分フォルダから大サイズのボード写真を収集）
python board_photo_collector.py "H:/path/to/photos" --pattern "10*" --size 大

# 閾値を指定（ボード比率0.5以上を収集）
python board_photo_collector.py "H:/path/to/photos" --threshold 0.5

# 重複除去をスキップ
python board_photo_collector.py "H:/path/to/photos" --no-dedup

# 出力ファイル名を指定
python board_photo_collector.py "H:/path/to/photos" --output my_results.json
```

### Pythonコードから使用

```python
from pathlib import Path
from board_photo_collector import create_collector

# コレクター作成
summarygenerator_root = Path(r"C:\path\to\summarygenerator")
collector = create_collector(summarygenerator_root)

# ボード写真を収集
base_dir = Path(r"H:\path\to\photos")
result = collector.collect_from_folders(
    base_dir=base_dir,
    folder_pattern="10*",  # 10月分フォルダ
    threshold=0.8,          # ボード比率0.8以上
    size_filter="大"        # 大サイズのみ
)

# 重複除去
result = collector.deduplicate_by_hash(result)

# JSON保存
collector.save_to_json(result, Path("output.json"))

print(f"収集完了: {result['total_board_photos']}枚")
```

## API

### BoardPhotoCollector

#### `__init__(yolo_predictor, board_classifier)`
コンストラクタ。

**引数:**
- `yolo_predictor`: YOLOPredictorインスタンス
- `board_classifier`: judge_caption_board_closeup関数

#### `scan_folder(folder, threshold=0.3, size_filter=None)`
フォルダ内のボード写真を検出。

**引数:**
- `folder` (Path): 対象フォルダ
- `threshold` (float): ボード面積比率の閾値（デフォルト: 0.3）
- `size_filter` (str | None): サイズフィルタ（"大"/"中"/"小"/"微小"）

**戻り値:**
- List[Dict]: ボード写真のリスト

#### `collect_from_folders(base_dir, folder_pattern="*", threshold=0.8, size_filter=None)`
複数フォルダからボード写真を収集。

**引数:**
- `base_dir` (Path): ベースディレクトリ
- `folder_pattern` (str): フォルダ検索パターン（例: "10*"）
- `threshold` (float): ボード面積比率の閾値
- `size_filter` (str | None): サイズフィルタ

**戻り値:**
- Dict: 収集結果

#### `deduplicate_by_hash(collection_result)`
ハッシュで重複を除去。

**引数:**
- `collection_result` (Dict): collect_from_foldersの結果

**戻り値:**
- Dict: 重複除去後の結果

#### `save_to_json(result, output_path)`
結果をJSONファイルに保存。

#### `load_from_json(input_path)`
JSONファイルから結果を読み込み（staticmethod）。

### ヘルパー関数

#### `create_collector(summarygenerator_root)`
BoardPhotoCollectorインスタンスを作成。

**引数:**
- `summarygenerator_root` (Path): summarygeneratorプロジェクトのルートパス

**戻り値:**
- BoardPhotoCollector: インスタンス

## ボードサイズ分類

| 分類 | ボード面積比率 | 用途 |
|------|---------------|------|
| 大 | 0.8以上 | OCRに最適、画面いっぱいのボード |
| 中 | 0.5-0.8未満 | ボードは明確だが周囲も写っている |
| 小 | 0.3-0.5未満 | ボードが小さめ |
| 微小 | 0.3未満 | ボードがほとんど見えない |

## 出力形式（JSON）

```json
{
  "base_directory": "H:/path/to/photos",
  "folder_pattern": "10*",
  "threshold": 0.8,
  "size_filter": "大",
  "total_folders": 17,
  "folders_with_boards": 11,
  "total_board_photos": 34,
  "duplicates_removed": 23,
  "folders": {
    "1003打換え": {
      "folder_path": "H:/path/to/photos/1003打換え",
      "board_count": 3,
      "photos": [
        {
          "path": "H:/path/to/photos/1003打換え/路盤出来形検測/No.0R/RIMG9202.JPG",
          "relative_path": "路盤出来形検測/No.0R/RIMG9202.JPG",
          "filename": "RIMG9202.JPG",
          "board_ratio": 0.9439,
          "board_size": "大",
          "size": 254832
        }
      ]
    }
  }
}
```

## 使用例

### 例1: 10月分の出来型ボード写真を収集

```bash
python board_photo_collector.py \
  "H:/マイドライブ/〇市道　上古閑大和第1号線舗装打換工事/８工事写真" \
  --pattern "10*" \
  --size 大 \
  --output october_boards.json
```

### 例2: 特定フォルダから中サイズ以上を収集

```bash
python board_photo_collector.py \
  "H:/path/to/project/photos" \
  --pattern "1007*" \
  --threshold 0.5 \
  --output 1007_boards.json
```

### 例3: Pythonから詳細制御

```python
from pathlib import Path
from board_photo_collector import create_collector, BoardPhotoCollector

# コレクター作成
collector = create_collector(Path(r"C:\path\to\summarygenerator"))

# カスタム収集
base_dir = Path(r"H:\project\photos")

# まず全フォルダをスキャン
result = collector.collect_from_folders(
    base_dir,
    folder_pattern="*",
    threshold=0.3,  # 低めの閾値で広く収集
    size_filter=None  # サイズフィルタなし
)

# カスタムフィルタリング（例: 路盤出来形検測のみ）
filtered_folders = {}
for name, data in result['folders'].items():
    filtered_photos = [
        p for p in data['photos']
        if '路盤出来形検測' in p['relative_path']
    ]
    if filtered_photos:
        filtered_folders[name] = {
            **data,
            'photos': filtered_photos,
            'board_count': len(filtered_photos)
        }

result['folders'] = filtered_folders
result['total_board_photos'] = sum(
    v['board_count'] for v in filtered_folders.values()
)

# 重複除去
result = collector.deduplicate_by_hash(result)

# 保存
collector.save_to_json(result, Path("custom_results.json"))
```

## トラブルシューティング

### YOLOモデルが見つからない

エラー: `FileNotFoundError: YOLOモデルが見つかりません`

**解決策:**
- summarygeneratorプロジェクトのパスを確認
- YOLOモデル（`runs/train/db_training/weights/best.pt`）の存在を確認

### ボードが検出されない

**原因:**
1. YOLOモデルが適切でない
2. 閾値が高すぎる
3. 画像が破損している

**解決策:**
- `--threshold 0.3` で閾値を下げて試す
- 画像をウィジェットで確認（`run_image_preview_dialog.py`）

### 重複が多すぎる

**原因:**
同じ画像が複数のフォルダに存在（自動分類フォルダなど）

**解決策:**
- 重複除去は自動で行われる（デフォルト有効）
- `--no-dedup` オプションで無効化可能

## 既存モジュールとの比較

| 機能 | list_dekigata_board_photos_yolo.py | board_photo_collector.py |
|------|-----------------------------------|--------------------------|
| YOLO検出 | ❌ 使用するが精度低い | ✅ 正しいモデルで高精度 |
| 汎用性 | ❌ 特定プロジェクト依存 | ✅ 汎用的 |
| 重複除去 | ❌ なし | ✅ ハッシュによる確実な除去 |
| フィルタリング | ❌ 固定 | ✅ 柔軟なオプション |
| 使いやすさ | ❌ コード修正必要 | ✅ コマンドラインで完結 |

## ライセンス

MIT License

## 作成日

2025-11-07

## 変更履歴

- v1.0.0 (2025-11-07): 初版リリース
  - YOLOによるボード検出
  - ボードサイズ分類（大・中・小・微小）
  - ハッシュによる重複除去
  - コマンドラインインターフェース
