export type Backend = 'auto' | 'webgpu' | 'webgl2';
export type Projection = 'bipartite' | 'clique' | 'star';
export interface Selection { vertices: string[]; hyperedges: string[] }
export interface ViewerSettings {
  layoutRunning: boolean;
  /** Integer from 1 to 16. */
  iterationsPerFrame: number;
  /** Requires a nonempty selection when enabling. */
  focus: boolean;
  attention: boolean;
  lasso: boolean;
  hulls: boolean;
  hideHubs: boolean;
  wireframe: boolean;
  /** 0–1. */ opacity: number;
  vertexLabels: boolean;
  hyperedgeLabels: boolean;
  labelMode: 'capped' | 'selection' | 'all';
  /** 0.1–10. */ labelScale: number;
  /** 0–1. */ labelVariation: number;
  /** 0.001–1. */ dt: number;
  /** 0–1. */ damping: number;
  /** 0–1,000,000. */ repulsion: number;
  /** 0–100. */ attraction: number;
  /** 0–100. */ gravity: number;
  /** 0.01–100,000. */ idealLength: number;
  /** 0.01–2. */ theta: number;
}
export interface ViewerState {
  /** Actual Bevy/wgpu adapter information for diagnostics. */
  renderer?: {backend: string; name: string; driver: string; deviceType: string};
  /** Camera metadata only; node positions stay inside Bevy. */
  camera?: {yaw: number | null; pitch: number | null; radius: number | null; focus: [number, number, number];
    target: {yaw: number; pitch: number; radius: number; focus: [number, number, number]}};
  projection: Projection;
  selection: Selection;
  graph: {id: string; title: string; vertices: number; hyperedges: number; links: number; iterations: number; warnings: string[]};
  settings: ViewerSettings;
}
export interface ViewerEvents {
  selectionchange: Selection;
  statechange: ViewerState;
  error: string;
}
export interface HyperViewer {
  /** The current canvas; startup fallback or a backend change may replace the supplied element. */
  readonly canvas: HTMLCanvasElement;
  readonly backend: Exclude<Backend, 'auto'>;
  loadJson(json: string): Promise<void>;
  setProjection(projection: Projection): Promise<void>;
  select(selection: Selection): Promise<void>;
  clearSelection(): Promise<void>;
  frame(): Promise<void>;
  setSettings(settings: Partial<ViewerSettings>): Promise<void>;
  /** Returns a detached snapshot of the latest acknowledged or published state. */
  getState(): ViewerState;
  on<K extends keyof ViewerEvents>(type: K, listener: (event: ViewerEvents[K]) => void): () => void;
  dispose(): Promise<void>;
}
export interface ViewerOptions {
  canvas: HTMLCanvasElement;
  backend?: Backend;
  /** Omit to disable persistence. Use different keys for independent sessions. */
  sessionKey?: string;
  /** Defaults to renderers.json beside the built SDK module. */
  manifestUrl?: string | URL;
}
export function createHyperViewer(options: ViewerOptions): Promise<HyperViewer>;
