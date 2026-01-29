# 解決済みIssue

## Issue #1: レイアウト追加後の日本語文字化け

### 症状
出来形モードでDXFエクスポート時、Pyodideでレイアウト追加後に日本語テキストが文字化けする。

### 原因
1. `recover.read()`がDXFのエンコーディングを`cp1252`と誤検出
2. 出力時に`dxf_out_str.encode('utf-8')`で固定エンコード → エンコーディング不一致

### 修正
**ファイル**: `static/pyodide_layouts.js` (173-178行)

```python
# 出力（doc.encode()でバイト列に変換）
out_buffer = io.StringIO()
doc.write(out_buffer)
dxf_out_str = out_buffer.getvalue()
dxf_out_bytes = doc.encode(dxf_out_str)  # ezdxfが正しいエンコーディングを使用
dxf_output_b64 = base64.b64encode(dxf_out_bytes).decode('ascii')
```

### 補足
- `doc.encode()`はezdxfが内部で`output_encoding`を使用し、適切なエラーハンドラ(`dxfreplace`)を適用
- DXF R2007以降は`output_encoding`が自動的にUTF-8になる
- 手動で`encode('utf-8')`するとエンコーディング不一致で文字化けする

---

## Issue #2: サイドバーUI部品が横に長すぎる

### 症状
サイドバー内のUI部品が1行に詰め込まれすぎて、サイドバーが横に広がる。

### 修正
**ファイル**: `src/app_ui.rs`

UI部品を3行に分割：
- 1行目: 列間隔 + 行間隔
- 2行目: 縦倍率 + 出来形スケール
- 3行目: チェックボックス + ページナビゲーション
