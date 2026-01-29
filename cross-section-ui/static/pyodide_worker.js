/**
 * Pyodide WebWorker - UIをブロックせずにezdxfを実行
 */

let pyodide = null;
let ezdxfReady = false;

// Pyodide初期化
async function initPyodide() {
    if (ezdxfReady) return;

    importScripts('https://cdn.jsdelivr.net/pyodide/v0.24.1/full/pyodide.js');
    pyodide = await loadPyodide();
    await pyodide.loadPackage('micropip');
    await pyodide.runPythonAsync(`
import micropip
await micropip.install('ezdxf')
import ezdxf
print('ezdxf ready in worker')
`);
    ezdxfReady = true;
    self.postMessage({ type: 'ready' });
}

// レイアウト追加処理
async function addLayouts(dxfBase64, scale, pageCount) {
    if (!ezdxfReady) {
        await initPyodide();
    }

    pyodide.globals.set('dxf_input_b64', dxfBase64);
    pyodide.globals.set('param_scale', scale);
    pyodide.globals.set('param_pages', pageCount);

    await pyodide.runPythonAsync(`
import ezdxf
from ezdxf import recover
import io
import base64

A3_WIDTH = 420.0
A3_HEIGHT = 297.0
PAGE_GAP_MM = 10.0

scale = param_scale
page_count = param_pages

# DXF読み込み
dxf_bytes = base64.b64decode(dxf_input_b64)
stream = io.BytesIO(dxf_bytes)
doc, auditor = recover.read(stream)
print(f"[Worker] DXF version: {doc.dxfversion}, encoding: {doc.encoding}")

# 計算
page_height_dxf = A3_HEIGHT * scale
page_gap_dxf = PAGE_GAP_MM * scale
frame_width_dxf = A3_WIDTH * scale

# レイアウト追加
layouts_created = []
for i in range(page_count):
    layout_name = f"Page_{i+1}"
    if layout_name in doc.layouts:
        continue

    y = i * (page_height_dxf + page_gap_dxf)
    layout = doc.layouts.new(layout_name)
    layout.page_setup(size=(A3_WIDTH, A3_HEIGHT), margins=(0,0,0,0), units="mm")

    vp_center = (A3_WIDTH/2, A3_HEIGHT/2)
    vp_size = (A3_WIDTH, A3_HEIGHT)
    view_center = (frame_width_dxf/2, y + page_height_dxf/2)
    view_height = page_height_dxf

    layout.add_viewport(
        center=vp_center,
        size=vp_size,
        view_center_point=(view_center[0], view_center[1], 0),
        view_height=view_height
    )
    layouts_created.append(layout_name)

    if i == 0 and "Layout1" in doc.layouts:
        doc.layouts.delete("Layout1")

# 出力
out_buffer = io.StringIO()
doc.write(out_buffer)
dxf_out_str = out_buffer.getvalue()
dxf_out_bytes = doc.encode(dxf_out_str)
dxf_output_b64 = base64.b64encode(dxf_out_bytes).decode('ascii')
print(f"[Worker] Created layouts: {layouts_created}")
`);

    const resultBase64 = pyodide.globals.get('dxf_output_b64');

    // クリーンアップ
    pyodide.globals.delete('dxf_input_b64');
    pyodide.globals.delete('param_scale');
    pyodide.globals.delete('param_pages');
    pyodide.globals.delete('dxf_output_b64');

    return resultBase64;
}

// メッセージハンドラ
self.onmessage = async (e) => {
    const { type, id, dxfBase64, scale, pageCount } = e.data;

    try {
        if (type === 'init') {
            await initPyodide();
        } else if (type === 'addLayouts') {
            const result = await addLayouts(dxfBase64, scale, pageCount);
            self.postMessage({ type: 'result', id, result });
        }
    } catch (error) {
        self.postMessage({ type: 'error', id, error: error.toString() });
    }
};
