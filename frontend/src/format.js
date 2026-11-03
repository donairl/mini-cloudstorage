export function formatSize(n) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024
    i++
  }
  return `${i ? n.toFixed(1) : n} ${units[i]}`
}

// SQLite CURRENT_TIMESTAMP is UTC "YYYY-MM-DD HH:MM:SS".
export function formatDate(s) {
  const d = new Date(s.replace(' ', 'T') + 'Z')
  return isNaN(d) ? s : d.toLocaleString()
}
