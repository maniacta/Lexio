/** Fired when knowledge points / sources change and sidebars should reload. */
export const DATA_CHANGED = "lexio:data-changed";

export function notifyDataChanged() {
  window.dispatchEvent(new CustomEvent(DATA_CHANGED));
}
