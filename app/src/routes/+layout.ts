// Static SPA inside Tauri: no server rendering; each route is prerendered as
// an empty shell so deep links load without relying on a fallback.
export const ssr = false;
export const prerender = true;
