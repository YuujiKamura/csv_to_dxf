/**
 * Pyodide + ezdxf でDXFにペーパースペースレイアウトを追加するモジュール
 */

// グローバル状態
window.pyodideLayouts = {
    pyodide: null,
    ezdxfReady: false,
    loading: false,
    error: null
};

/**
 * Pyodideとezdxfを初期化（バックグラウンドで呼び出し）
 */
async function initPyodideLayouts() {
    if (window.pyodideLayouts.loading || window.pyodideLayouts.ezdxfReady) {
        return;
    }

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
        console.error('[PyodideLayouts] エラー:', e);
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
    if (!window.pyodideLayouts.ezdxfReady) {
        console.warn('[PyodideLayouts] ezdxf未準備、元のDXFを返します');
        return dxfBytes;
    }

    console.log(`[PyodideLayouts] レイアウト追加: scale=${scale}, pages=${pageCount}`);

    const pyodide = window.pyodideLayouts.pyodide;

    // DXFデータをPython側に渡す（大きなファイル対応）
    // チャンクに分けてBase64変換（スタックオーバーフロー回避）
    const CHUNK_SIZE = 32768;
    let binaryString = '';
    for (let i = 0; i < dxfBytes.length; i += CHUNK_SIZE) {
        const chunk = dxfBytes.subarray(i, Math.min(i + CHUNK_SIZE, dxfBytes.length));
        binaryString += String.fromCharCode.apply(null, chunk);
    }
    const dxfBase64 = btoa(binaryString);
    pyodide.globals.set('dxf_input_b64', dxfBase64);
    pyodide.globals.set('param_scale', scale);
    pyodide.globals.set('param_pages', pageCount);

    await pyodide.runPythonAsync(`
import ezdxf
import io
import base64

A3_WIDTH = 420
A3_HEIGHT = 297
PAGE_GAP_MM = 10.0

scale = param_scale
page_count = param_pages

# DXF読み込み
dxf_bytes = base64.b64decode(dxf_input_b64)
dxf_str = dxf_bytes.decode('utf-8')
stream = io.StringIO(dxf_str)
doc = ezdxf.read(stream)

# 計算
page_height_dxf = A3_HEIGHT * scale
page_gap_dxf = PAGE_GAP_MM * scale
frame_width_dxf = A3_WIDTH * scale

# レイアウト追加
for i in range(page_count):
    layout_name = f"Page_{i+1}"
    if layout_name in doc.layouts:
        continue

    y = i * (page_height_dxf + page_gap_dxf)
    layout = doc.layouts.new(layout_name)
    layout.page_setup(size=(A3_WIDTH, A3_HEIGHT), margins=(0,0,0,0), units="mm")
    layout.add_viewport(
        center=(A3_WIDTH/2, A3_HEIGHT/2),
        size=(A3_WIDTH, A3_HEIGHT),
        view_center_point=(frame_width_dxf/2, y + page_height_dxf/2, 0),
        view_height=page_height_dxf
    )

    # 最初のレイアウト追加後にLayout1を削除
    if i == 0 and "Layout1" in doc.layouts:
        doc.layouts.delete("Layout1")

# 出力
out_buffer = io.StringIO()
doc.write(out_buffer)
dxf_out_str = out_buffer.getvalue()
dxf_out_bytes = dxf_out_str.encode('utf-8')
dxf_output_b64 = base64.b64encode(dxf_out_bytes).decode('ascii')
print(f"Layout added: {page_count} pages")
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
