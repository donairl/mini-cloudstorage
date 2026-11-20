<script setup>
import { version } from '../../package.json'
</script>

<template>
  <div class="page">
    <header class="topbar">
      <strong class="brand">Mini-Drive</strong>
      <RouterLink to="/">Back to files</RouterLink>
    </header>

    <main class="content">
      <h1>About Mini-Drive</h1>
      <p class="muted">Version {{ version }}</p>
      <p>
        Mini-Drive is a self-hosted file storage app. Your files stay on your own server: the files themselves on local
        disk, their metadata in SQLite.
      </p>

      <h2>Features</h2>
      <ul>
        <li>Upload, download, rename and organise files in folders</li>
        <li>Search files and folders by name</li>
        <li>Preview images, video, audio, PDF and text files in the browser</li>
        <li>Share a file or a whole folder with a link, with optional password and expiry</li>
        <li>Download a folder as a .zip</li>
        <li>Trash with restore</li>
        <li>Storage quota and usage statistics</li>
      </ul>

      <h2>Built with</h2>
      <p>Rust (Axum) and SQLite on the backend, Vue 3 on the frontend.</p>
    </main>
  </div>
</template>

<style scoped>
.topbar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.brand { margin-right: auto; }
.content { max-width: 700px; margin: 0 auto; padding: 1rem; line-height: 1.6; }
h1 { margin: 0; font-size: 1.25rem; }
h2 { margin: 1.5rem 0 0.25rem; font-size: 1rem; }
.muted { margin: 0.25rem 0 1rem; color: var(--muted); font-size: 0.9rem; }
ul { margin: 0; padding-left: 1.25rem; }
</style>
