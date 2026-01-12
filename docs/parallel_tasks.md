# 縦断図実装 - 並列タスク分割

以下の3タスクは相互に依存せず、並列実行可能。

---

## Task 1: データ構造とX座標計算 (Issue #22)

### 対象ファイル
`cross-section-ui/src/lib.rs`

### 実装内容

1. `CrossSectionData` 構造体に `route_distance: f64` フィールドを追加（縦断図用の路線方向距離）

2. `parse_station_distance()` 関数を修正:
   - 現状: `No.X` → `X * 10.0`
   - 修正後: `No.X` → `X * 20.0`（測点間隔は通常20m）

3. `generate_longitudinal_drawing()` 内のX座標計算を確認:
   - `route_distance` フィールドがあれば優先使用
   - なければ `parse_station_distance()` で近似

4. Y座標（標高）の計算確認:
   - `(elevation - dl) * scale_y` が正しく適用されているか確認

### 完了条件
- サンプルデータで縦断図のX軸が20m間隔で正しく配置される
- `trunk build --release` が成功する

---

## Task 2: データ表の書式 (Issue #21)

### 対象ファイル
`cross-section-ui/src/lib.rs`

### 実装内容

1. 回転テキスト関数を追加:
```rust
fn add_text_rotated(drawing: &mut Drawing, x: f64, y: f64, text: &str,
                    height: f64, color: i16, layer: &str, align: TextAlign, rotation: f64)
```

2. `generate_longitudinal_drawing()` のデータ表部分を修正:

| 行 | 内容 | 回転 | 配置 |
|----|------|------|------|
| 勾配 | slope% | 0° | 上寄せ |
| 盛土 | fill値 | 0° | 中央 |
| 切土 | cut値 | 0° | 中央 |
| 計画高 | FH | 90° | 中央 |
| 地盤高 | GH | 90° | 中央 |
| 追加距離 | cum_dist | 90° | 中央 |
| 単距離 | unit_dist | 0° | 中央 |
| 測点名 | name | 0° | 下寄せ |

3. 行高さを `row_height = 350.0` に設定（回転テキスト対応）

4. 数値フォーマット:
   - 計画高/地盤高: `{:.3}` (例: 10.123)
   - 追加距離: `{:.3}` (例: 40.000)
   - 単距離: `{:.2}` (例: 20.00)
   - 勾配: `{:.3}%` (例: 1.234%)

### 完了条件
- 表の各行が適切な高さで表示される
- 計画高、地盤高、追加距離が90度回転で表示される
- `trunk build --release` が成功する

---

## Task 3: グラフ注記 (Issue #20)

### 対象ファイル
`cross-section-ui/src/lib.rs`

### 実装内容

1. `generate_longitudinal_drawing()` のグラフ部分に以下を追加:

2. 勾配変化点の注記（計画線の折れ点）:
```
   ΔV=+0.123
   i=1.5%
   L=20.00m
```
   - ΔV: 前後の勾配差
   - i: その区間の勾配(%)
   - L: 区間長(m)

3. 測点ラベル（グラフ上端に測点名を表示）

4. タイトルブロック（図面左上または右下）:
```
縦断図
縮尺 H=1:500 V=1:100
```

5. 起終点の標高ラベル（グラフの左端・右端）

### 完了条件
- 勾配変化点に注記が表示される
- タイトルとスケールが表示される
- `trunk build --release` が成功する

---

## 共通事項

### ビルド方法
```bash
cd cross-section-ui
trunk build --release
```

### テスト方法
```bash
trunk serve --port 9008
# ブラウザで http://localhost:9008 を開き「縦断」ボタンをクリック
```

### 参考PDF
`H:\マイドライブ\過去の現場_元請\〇市道　上古閑大和第1号線舗装打換工事\３設計照査\７縦断図.pdf`

### 注意
- 各タスクは独立して実装可能
- 既存の `add_text()` 関数は変更せず、新規関数を追加する形で実装
- DXFバージョンは `AcadVersion::R2010` を維持
