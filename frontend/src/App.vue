<script setup>
import AppDialog from './components/AppDialog.vue'
import ToastHost from './components/ToastHost.vue'
</script>

<template>
  <RouterView />
  <AppDialog />
  <ToastHost />
</template>

<style>
:root {
  --bg: #f6f7f9;
  --surface: #fff;
  --text: #1f2328;
  --muted: #656d76;
  --border: #d8dee4;
  --accent: #0969da;
  --danger: #cf222e;
  font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
  color: var(--text);
  background: var(--bg);
}
* { box-sizing: border-box; }
body { margin: 0; }
button {
  font: inherit;
  padding: 0.4rem 0.8rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text);
  cursor: pointer;
}
button:hover { background: var(--bg); }
button.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
button.primary:hover { filter: brightness(1.1); }
button:disabled { opacity: 0.6; cursor: default; }
input {
  font: inherit;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: 6px;
}
a { color: var(--accent); }
:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.error { color: var(--danger); margin: 0.5rem 0; }
</style>
