export interface View {
  k: number;
  tx: number;
  ty: number;
}

export const VIEW_FIT: View = { k: 1, tx: 0, ty: 0 };
export const ZOOM_MIN = 1;
export const ZOOM_MAX = 200;
export const ZOOM_STEP = 1.5;

export function zoomAt(v: View, factor: number, mx: number, my: number): View {
  const k = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, v.k * factor));
  if (k === ZOOM_MIN) return VIEW_FIT;
  const r = k / v.k;
  return { k, tx: mx - (mx - v.tx) * r, ty: my - (my - v.ty) * r };
}

export function pan(v: View, dx: number, dy: number): View {
  if (v.k === ZOOM_MIN) return v;
  return { ...v, tx: v.tx + dx, ty: v.ty + dy };
}

export function apply(v: View, sx: number, sy: number): [number, number] {
  return [sx * v.k + v.tx, sy * v.k + v.ty];
}
