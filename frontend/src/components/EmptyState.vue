<script setup>
defineEmits(['upload', 'new-folder'])
</script>

<template>
  <section class="empty-state">
    <svg class="art" width="120" height="96" viewBox="0 0 120 96" fill="none" aria-hidden="true">
      <path d="M12 26a6 6 0 0 1 6-6h26l8 9h50a6 6 0 0 1 6 6v45a6 6 0 0 1-6 6H18a6 6 0 0 1-6-6V26Z" fill="var(--bg)" stroke="var(--border)" stroke-width="2" />
      <path d="M12 38h96v42a6 6 0 0 1-6 6H18a6 6 0 0 1-6-6V38Z" fill="var(--surface)" stroke="var(--border)" stroke-width="2" />
      <path d="M47 70a9 9 0 0 1-1.2-17.9A12 12 0 0 1 69 49a8 8 0 0 1 4 15" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" fill="none" />
      <path d="M60 58v18M54 64l6-6 6 6" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
      <circle cx="98" cy="12" r="3" fill="var(--accent)" opacity="0.35" />
      <circle cx="20" cy="10" r="2" fill="var(--muted)" opacity="0.4" />
      <circle cx="110" cy="28" r="2" fill="var(--muted)" opacity="0.4" />
    </svg>
    <h2>This folder is empty</h2>
    <p>Drag and drop files anywhere on this page, or use the buttons below.</p>
    <div class="buttons">
      <button type="button" class="primary" @click="$emit('upload')">Upload files</button>
      <button type="button" @click="$emit('new-folder')">New folder</button>
    </div>
  </section>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  max-width: 32rem;
  margin: 2rem auto;
  padding: 2.5rem 1.5rem;
  border: 2px dashed var(--border);
  border-radius: 12px;
  background: var(--surface);
}
.art { margin-bottom: 1rem; }
h2 { margin: 0 0 0.5rem; font-size: 1.2rem; }
p { margin: 0 0 1.5rem; color: var(--muted); max-width: 26rem; line-height: 1.5; }
.buttons { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.5rem; }
@media (max-width: 400px) {
  .empty-state { padding: 2rem 1rem; margin: 1rem 0; }
  .buttons { flex-direction: column; width: 100%; }
}
</style>
