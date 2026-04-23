// SvelteKit prerenders the SPA shell at build time and Tauri serves it as
// a single index.html. No SSR runtime ships with the app.
export const prerender = true;
export const ssr = false;
