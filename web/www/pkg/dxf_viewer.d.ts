/* tslint:disable */
/* eslint-disable */

/**
 * Chroma subsampling format
 */
export enum ChromaSampling {
  /**
   * Both vertically and horizontally subsampled.
   */
  Cs420 = 0,
  /**
   * Horizontally subsampled.
   */
  Cs422 = 1,
  /**
   * Not subsampled.
   */
  Cs444 = 2,
  /**
   * Monochrome.
   */
  Cs400 = 3,
}

export class DxfViewer {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get result as JSON
   */
  get_result_json(): string;
  /**
   * Set canvas size
   */
  set_canvas_size(width: number, height: number): void;
  /**
   * Create a new DXF viewer
   */
  constructor();
  /**
   * Pan the view
   */
  pan(dx: number, dy: number): void;
  /**
   * Zoom the view
   */
  zoom(factor: number, center_x: number, center_y: number): void;
  /**
   * Render to canvas
   */
  render(ctx: CanvasRenderingContext2D): void;
  /**
   * Load and parse DXF content using the dxf crate
   */
  load_dxf(content: string): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_dxfviewer_free: (a: number, b: number) => void;
  readonly dxfviewer_get_result_json: (a: number) => [number, number, number, number];
  readonly dxfviewer_load_dxf: (a: number, b: number, c: number) => [number, number, number, number];
  readonly dxfviewer_new: () => number;
  readonly dxfviewer_pan: (a: number, b: number, c: number) => void;
  readonly dxfviewer_render: (a: number, b: any) => [number, number];
  readonly dxfviewer_set_canvas_size: (a: number, b: number, c: number) => void;
  readonly dxfviewer_zoom: (a: number, b: number, c: number, d: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
