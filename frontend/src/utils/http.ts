export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

export interface HttpOptions {
  method?: HttpMethod
  data?: unknown
  headers?: Record<string, string>
  timeoutMs?: number
}

export async function httpJson<T>(url: string, options: HttpOptions = {}): Promise<T> {
  const { method = 'GET', data, headers, timeoutMs } = options
  const controller = new AbortController()
  const timeoutId = timeoutMs ? setTimeout(() => controller.abort(), timeoutMs) : undefined

  try {
    const response = await fetch(url, {
      method,
      headers: {
        'Content-Type': 'application/json',
        ...headers
      },
      body: data !== undefined ? JSON.stringify(data) : undefined,
      signal: controller.signal
    })

    if (!response.ok) {
      const message = await response.text().catch(() => '')
      throw new Error(message || `Request failed with status ${response.status}`)
    }

    if (response.status === 204) {
      return null as T
    }

    const contentType = response.headers.get('content-type') || ''
    if (!contentType.includes('application/json')) {
      throw new Error('Expected JSON response')
    }

    return (await response.json()) as T
  } finally {
    if (timeoutId) {
      clearTimeout(timeoutId)
    }
  }
}
