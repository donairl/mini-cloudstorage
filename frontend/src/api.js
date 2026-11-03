function parse(text) {
  try {
    return text ? JSON.parse(text) : null
  } catch {
    return null
  }
}

function toLoginOn401(status, url) {
  if (status === 401 && !url.startsWith('/api/auth/')) window.location.assign('/login')
}

async function request(method, url, body) {
  const res = await fetch(url, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  })
  const data = parse(await res.text())
  toLoginOn401(res.status, url)
  if (!res.ok) throw new Error(data?.error || `Request failed (${res.status})`)
  return data
}

export const get = (url) => request('GET', url)
export const post = (url, body) => request('POST', url, body)
export const patch = (url, body) => request('PATCH', url, body)
export const del = (url) => request('DELETE', url)

// fetch has no upload progress, so uploads use XMLHttpRequest.
export function upload(file, folderId, onProgress) {
  return new Promise((resolve, reject) => {
    const url = '/api/files/upload' + (folderId ? `?folder_id=${encodeURIComponent(folderId)}` : '')
    const form = new FormData()
    form.append('file', file)
    const xhr = new XMLHttpRequest()
    xhr.open('POST', url)
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable && onProgress) onProgress(e.loaded / e.total)
    }
    xhr.onload = () => {
      const data = parse(xhr.responseText)
      toLoginOn401(xhr.status, url)
      if (xhr.status >= 200 && xhr.status < 300) resolve(data)
      else {
        const fallback = { 413: 'File too large', 507: 'Storage full — free up space or upgrade your plan' }
        reject(new Error(data?.error || fallback[xhr.status] || `Upload failed (${xhr.status})`))
      }
    }
    xhr.onerror = () => reject(new Error('Network error'))
    xhr.send(form)
  })
}
