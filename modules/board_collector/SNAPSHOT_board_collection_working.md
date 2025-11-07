# ボード写真収集スナップショット

**日時**: 2025-11-07
**状態**: 動作確認済み

## 成果物

### 1. ボード写真収集スクリプト
**ファイル**: `collect_october_board_photos.py`

#### 特徴
- 正しいYOLOモデル使用: `runs/train/db_training/weights/best.pt`
- ボードサイズ分類: 大（0.8以上）、中（0.5-0.8）、小（0.3-0.5）、微小（0.3未満）
- クラス名正規化: `caption_board_dekigata`などに対応
- コマンドラインオプション:
  - `--model`: YOLOモデルパス
  - `--threshold`: ボード面積比率の閾値（デフォルト: 0.8）
  - `--size`: 特定サイズのみ収集（大/中/小/微小）
  - `--output`: 出力JSONファイル名

#### 実行例
```bash
# 大サイズのみ収集
python collect_october_board_photos.py --size 大 --output october_board_photos_large.json

# 中サイズ以上を収集
python collect_october_board_photos.py --threshold 0.5
```

#### 収集結果
- 対象: 10月分フォルダ（17フォルダ）
- 検出: 57枚（重複除去前）

### 2. 重複除去スクリプト
**ファイル**: `deduplicate_board_photos.py`

#### 特徴
- MD5ハッシュによる重複検出
- 異なるフォルダの同一ファイルを除去
- 元ファイル情報を保持

#### 実行例
```bash
python deduplicate_board_photos.py october_board_photos_large.json
# 出力: october_board_photos_large_dedup.json
```

#### 重複除去結果
- 元: 57枚
- 重複: 23枚
- 最終: 34枚

### 3. テストスクリプト
**ファイル**: `test_single_folder_board_collection.py`

1つのフォルダで動作確認するための小規模テストスクリプト。

## 重要な発見

### 問題1: YOLOモデルの誤用
**間違い**: `datasets/board_detection_run/weights/best.pt`を使用
**正しい**: `runs/train/db_training/weights/best.pt`を使用

古いモデルはボード全体を検出できず、部分的な検出しかできなかった。

### 問題2: クラス名の不一致
YOLOが返すクラス名:
- `caption_board_dekigata` (出来型ボード)
- `caption_board_thermometer` (温度管理ボード)

正規化ロジックで対応:
```python
cname = bbox.get('cname', '')
if any(keyword in cname for keyword in ['出来形', '出来型', 'caption_board', 'dekigata']):
    bbox['cname'] = 'caption_board'
    bbox['role'] = 'caption_board'
```

### 問題3: 重複ファイル
同じ画像が異なるフォルダに存在（自動振り分けテスト、自動分類など）。
→ ハッシュによる重複除去で解決。

## 収集結果の内訳

### フォルダ別（重複除去後）
- 1003打換え: 3枚
- 1006打換え: 3枚
- 1007打換え: 6枚（As規制、プラント管理含む）
- 1008打換え: 4枚
- 1009打換え: 3枚
- 1010打換え: 5枚
- 1014区画線、管理: 10枚

**合計: 34枚**

## 次のステップ

1. ✅ ボード写真収集の自動化
2. ✅ 正しいYOLOモデルの使用
3. ✅ 重複除去
4. ⬜ データベースへの登録
5. ⬜ OCR処理
6. ⬜ 測定値の抽出

## 既存の駄目なモジュールからの脱却

### 従来のアプローチ（list_dekigata_board_photos_yolo.py）
- 各No.フォルダの「最後の写真」を機械的に選択
- YOLOは使わず、ファイル名ソート順に依存
- フォルダ構造の想定が固定（subfolder/flat）
- 特定プロジェクトに強く依存

### 新しいアプローチ（collect_october_board_photos.py）
- ✅ 正しいYOLOモデルでボード全体を検出
- ✅ ボード面積比率による客観的な判定
- ✅ コマンドラインオプションで柔軟に設定可能
- ✅ ハッシュによる確実な重複除去
- ✅ 複雑なフォルダ構造にも対応（再帰探索）

## ファイルリスト

- `collect_october_board_photos.py` - メインスクリプト
- `deduplicate_board_photos.py` - 重複除去
- `test_single_folder_board_collection.py` - テスト用
- `october_board_photos_large_correct.json` - 収集結果（重複除去前）
- `october_board_photos_large_correct_dedup.json` - 収集結果（重複除去後）
- `SNAPSHOT_board_collection_working.md` - このファイル

## 教訓

1. **YOLOモデルの選択が重要**: 検出精度は学習データに依存
2. **ハッシュによる重複除去**: ファイル名だけでは不十分
3. **小さく始める**: 1つのフォルダでテストしてから全体を処理
4. **可視化ウィジェットの活用**: 実際の画像を見て検証することが重要
5. **汎用性と特化のバランス**: プロジェクト固有の処理は分離する
