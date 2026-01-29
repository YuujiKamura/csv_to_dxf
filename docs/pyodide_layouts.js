/**
 * Pyodide + ezdxf でDXFにペーパースペースレイアウトを追加するモジュール
 */

// グローバル状態
window.pyodideLayouts = {
    pyodide: null,
    ezdxfReady: false,
    loading: false,
    error: null,
    initPromise: null  // 競合状態防止用
};

/**
 * Pyodideとezdxfを初期化（バックグラウンドで呼び出し）
 */
async function initPyodideLayouts() {
    // 競合状態防止: 既に初期化中または完了なら同じPromiseを返す
    if (window.pyodideLayouts.initPromise) {
        return window.pyodideLayouts.initPromise;
    }
    if (window.pyodideLayouts.ezdxfReady) {
        return Promise.resolve();
    }

    window.pyodideLayouts.initPromise = doInitPyodideLayouts();
    return window.pyodideLayouts.initPromise;
}

async function doInitPyodideLayouts() {
    window.pyodideLayouts.loading = true;
    console.log('[PyodideLayouts] 初期化開始...');

    try {
        // Pyodide読み込み
        const script = document.createElement('script');
        script.src = 'https://cdn.jsdelivr.net/pyodide/v0.24.1/full/pyodide.js';

        await new Promise((resolve, reject) => {
            script.onload = resolve;
            script.onerror = reject;
            document.head.appendChild(script);
        });

        console.log('[PyodideLayouts] Pyodideスクリプト読み込み完了');

        window.pyodideLayouts.pyodide = await loadPyodide();
        console.log('[PyodideLayouts] Pyodide初期化完了');

        // micropipとezdxfインストール
        await window.pyodideLayouts.pyodide.loadPackage('micropip');

        await window.pyodideLayouts.pyodide.runPythonAsync(`
import os
os.environ['EZDXF_DISABLE_C_EXT'] = '1'
import micropip
await micropip.install('ezdxf')
import ezdxf
print('ezdxf ready')
        `);

        window.pyodideLayouts.ezdxfReady = true;
        console.log('[PyodideLayouts] ezdxf準備完了');

    } catch (e) {
        window.pyodideLayouts.error = e.toString();
        window.pyodideLayouts.initPromise = null;  // リトライ可能にする
        console.error('[PyodideLayouts] エラー:', e);
        throw e;
    } finally {
        window.pyodideLayouts.loading = false;
    }
}

/**
 * DXFにレイアウトを追加
 * @param {Uint8Array} dxfBytes - 元のDXFバイナリ
 * @param {number} scale - スケール（例: 50 = 1:50）
 * @param {number} pageCount - ページ数
 * @returns {Promise<Uint8Array>} - レイアウト追加後のDXFバイナリ
 */
async function addLayoutsToDxf(dxfBytes, scale, pageCount) {
    // 引数検証
    if (pageCount <= 0) {
        console.warn('[PyodideLayouts] pageCountが0以下、元のDXFを返します');
        return dxfBytes;
    }

    if (!window.pyodideLayouts.ezdxfReady) {
        console.warn('[PyodideLayouts] ezdxf未準備、元のDXFを返します');
        return dxfBytes;
    }

    console.log(`[PyodideLayouts] レイアウト追加: scale=${scale}, pages=${pageCount}`);

    const pyodide = window.pyodideLayouts.pyodide;

    try {
        // DXFデータをPython側に渡す（大きなファイル対応）
        // チャンクに分けてBase64変換（スタックオーバーフロー回避）
        const CHUNK_SIZE = 32768;
        const chunks = [];
        for (let i = 0; i < dxfBytes.length; i += CHUNK_SIZE) {
            const chunk = dxfBytes.subarray(i, Math.min(i + CHUNK_SIZE, dxfBytes.length));
            chunks.push(String.fromCharCode.apply(null, chunk));
        }
        const dxfBase64 = btoa(chunks.join(''));
        pyodide.globals.set('dxf_input_b64', dxfBase64);
        pyodide.globals.set('param_scale', scale);
        pyodide.globals.set('param_pages', pageCount);

        await pyodide.runPythonAsync(`
import ezdxf
from ezdxf import recover
import io
import base64

A3_WIDTH = 420
A3_HEIGHT = 297
PAGE_GAP_MM = 10.0

scale = param_scale
page_count = param_pages

print(f"[Python] scale={scale}, page_count={page_count}")

# DXF読み込み（recoverモードでハンドル重複を修復）
dxf_bytes = base64.b64decode(dxf_input_b64)
dxf_str = dxf_bytes.decode('utf-8')
stream = io.StringIO(dxf_str)

try:
    doc, auditor = recover.read(stream)
except Exception as e:
    raise RuntimeError(f"DXF読み込みに失敗: {e}")

# エラー詳細のログ出力
if auditor.errors:
    print(f"[Python] DXF version: {doc.dxfversion}, errors fixed: {len(auditor.errors)}")
    for error in auditor.errors[:5]:  # 最初の5件のみ表示
        print(f"[Python]   - {error}")
    if len(auditor.errors) > 5:
        print(f"[Python]   ... and {len(auditor.errors) - 5} more")
else:
    print(f"[Python] DXF version: {doc.dxfversion}, no errors")


# 計算
page_height_dxf = A3_HEIGHT * scale
page_gap_dxf = PAGE_GAP_MM * scale
frame_width_dxf = A3_WIDTH * scale

print(f"[Python] page_height_dxf={page_height_dxf}, frame_width_dxf={frame_width_dxf}")

# レイアウト追加
layouts_created = []
for i in range(page_count):
    layout_name = f"Page_{i+1}"
    if layout_name in doc.layouts:
        print(f"[Python] {layout_name} already exists, skip")
        continue

    y = i * (page_height_dxf + page_gap_dxf)
    layout = doc.layouts.new(layout_name)
    layout.page_setup(size=(A3_WIDTH, A3_HEIGHT), margins=(0,0,0,0), units="mm")

    vp_center = (A3_WIDTH/2, A3_HEIGHT/2)
    vp_size = (A3_WIDTH, A3_HEIGHT)
    view_center = (frame_width_dxf/2, y + page_height_dxf/2)
    view_height = page_height_dxf

    print(f"[Python] {layout_name}: y={y}, view_center={view_center}, view_height={view_height}")

    layout.add_viewport(
        center=vp_center,
        size=vp_size,
        view_center_point=(view_center[0], view_center[1], 0),
        view_height=view_height
    )
    layouts_created.append(layout_name)

    # 最初のレイアウト追加後にLayout1を削除
    if i == 0 and "Layout1" in doc.layouts:
        doc.layouts.delete("Layout1")
        print("[Python] Layout1 deleted")

# 出力
out_buffer = io.StringIO()
doc.write(out_buffer)
dxf_out_str = out_buffer.getvalue()
dxf_out_bytes = dxf_out_str.encode('utf-8')
dxf_output_b64 = base64.b64encode(dxf_out_bytes).decode('ascii')

# 最終レイアウト一覧
final_layouts = [l.name for l in doc.layouts]
print(f"[Python] Created layouts: {layouts_created}")
print(f"[Python] Final layouts in DXF: {final_layouts}")
        `);

        // 結果を取得
        const resultBase64 = await pyodide.runPythonAsync('dxf_output_b64');
        const binaryString = atob(resultBase64);
        const resultBytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
            resultBytes[i] = binaryString.charCodeAt(i);
        }

        console.log('[PyodideLayouts] レイアウト追加完了');
        return resultBytes;

    } catch (e) {
        console.error('[PyodideLayouts] レイアウト追加エラー:', e);
        throw e;  // Rust側に伝播させる

    } finally {
        // Pythonグローバル変数のクリーンアップ（メモリリーク防止）
        try {
            pyodide.globals.delete('dxf_input_b64');
            pyodide.globals.delete('param_scale');
            pyodide.globals.delete('param_pages');
            pyodide.globals.delete('dxf_output_b64');
        } catch (cleanupError) {
            // クリーンアップ失敗は無視
        }
    }
}

/**
 * ezdxfの準備状態を確認
 * @returns {boolean}
 */
function isEzdxfReady() {
    return window.pyodideLayouts.ezdxfReady;
}

/**
 * 読み込み中かどうか
 * @returns {boolean}
 */
function isEzdxfLoading() {
    return window.pyodideLayouts.loading;
}

// グローバルに公開
window.initPyodideLayouts = initPyodideLayouts;
window.addLayoutsToDxf = addLayoutsToDxf;
window.isEzdxfReady = isEzdxfReady;
window.isEzdxfLoading = isEzdxfLoading;
