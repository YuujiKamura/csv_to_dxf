/**
 * Pyodide + ezdxf WebWorker版
 * UIをブロックせずにレイアウト追加を実行
 */

// グローバル状態
window.pyodideLayouts = {
    worker: null,
    ready: false,
    loading: false,
    pendingRequests: new Map(),
    requestId: 0
};

/**
 * WebWorkerを初期化
 */
function initPyodideLayouts() {
    if (window.pyodideLayouts.worker) return;

    console.log('[PyodideLayouts] WebWorker初期化...');
    window.pyodideLayouts.loading = true;

    const worker = new Worker('pyodide_worker.js');
    window.pyodideLayouts.worker = worker;

    worker.onmessage = (e) => {
        const { type, id, result, error } = e.data;

        if (type === 'ready') {
            window.pyodideLayouts.ready = true;
            window.pyodideLayouts.loading = false;
            console.log('[PyodideLayouts] WebWorker準備完了');
        } else if (type === 'result') {
            const pending = window.pyodideLayouts.pendingRequests.get(id);
            if (pending) {
                pending.resolve(result);
                window.pyodideLayouts.pendingRequests.delete(id);
            }
        } else if (type === 'error') {
            const pending = window.pyodideLayouts.pendingRequests.get(id);
            if (pending) {
                pending.reject(new Error(error));
                window.pyodideLayouts.pendingRequests.delete(id);
            }
        }
    };

    worker.onerror = (e) => {
        console.error('[PyodideLayouts] Worker error:', e);
        window.pyodideLayouts.loading = false;
    };

    // 初期化開始
    worker.postMessage({ type: 'init' });
}

/**
 * DXFにレイアウトを追加
 */
async function addLayoutsToDxf(dxfBytes, scale, pageCount) {
    if (pageCount <= 0) {
        console.warn('[PyodideLayouts] pageCountが0以下、元のDXFを返します');
        return dxfBytes;
    }

    // Workerがなければ初期化して待機
    if (!window.pyodideLayouts.worker) {
        initPyodideLayouts();
    }

    // 準備完了を待つ
    if (!window.pyodideLayouts.ready) {
        console.log('[PyodideLayouts] Worker準備待ち...');
        await new Promise((resolve) => {
            const check = () => {
                if (window.pyodideLayouts.ready) {
                    resolve();
                } else {
                    setTimeout(check, 100);
                }
            };
            check();
        });
    }

    console.log(`[PyodideLayouts] レイアウト追加: scale=${scale}, pages=${pageCount}`);

    // Base64エンコード
    const CHUNK_SIZE = 32768;
    const chunks = [];
    for (let i = 0; i < dxfBytes.length; i += CHUNK_SIZE) {
        const chunk = dxfBytes.subarray(i, Math.min(i + CHUNK_SIZE, dxfBytes.length));
        chunks.push(String.fromCharCode.apply(null, chunk));
    }
    const dxfBase64 = btoa(chunks.join(''));

    // リクエスト送信
    const id = ++window.pyodideLayouts.requestId;
    return new Promise((resolve, reject) => {
        window.pyodideLayouts.pendingRequests.set(id, {
            resolve: (resultBase64) => {
                // Base64デコード
                const binaryString = atob(resultBase64);
                const resultBytes = new Uint8Array(binaryString.length);
                for (let i = 0; i < binaryString.length; i++) {
                    resultBytes[i] = binaryString.charCodeAt(i);
                }
                console.log('[PyodideLayouts] レイアウト追加完了');
                resolve(resultBytes);
            },
            reject
        });

        window.pyodideLayouts.worker.postMessage({
            type: 'addLayouts',
            id,
            dxfBase64,
            scale,
            pageCount
        });
    });
}

function isEzdxfReady() {
    return window.pyodideLayouts.ready;
}

function isEzdxfLoading() {
    return window.pyodideLayouts.loading;
}

// グローバルに公開
window.initPyodideLayouts = initPyodideLayouts;
window.addLayoutsToDxf = addLayoutsToDxf;
window.isEzdxfReady = isEzdxfReady;
window.isEzdxfLoading = isEzdxfLoading;
