export type IndexSource = 'all' | 'hackernews' | 'lobsters'

export interface IndexCandle {
  date: string
  hour: number
  open: number | null
  high: number | null
  low: number | null
  close: number | null
}

// Global (/api/index) returns just candles; per-source adds `source`.
export interface IndexCandles {
  source?: string
  candles: IndexCandle[]
}

export interface Post {
  id: string
  title: string
  comments_url: string
  external_url: string | null
  posted_at: string
  comments: number
  score: number
  index: number | null
}

export interface Snapshot {
  source: string
  captured_at: string
  posts: Post[]
}

export type SourceOnly = Exclude<IndexSource, 'all'>

export async function fetchSnapshot(source: SourceOnly): Promise<Snapshot> {
  const res = await fetch(`/api/snapshot/${source}`)
  if (!res.ok) {
    throw new Error(`GET /api/snapshot/${source} -> ${res.status}`)
  }
  return (await res.json()) as Snapshot
}

// Global lives at /api/index; each source at /api/index/<source>.
export async function fetchIndexCandles(
  source: IndexSource,
  fromIso?: string,
  toIso?: string,
): Promise<IndexCandles> {
  const params = new URLSearchParams()
  if (fromIso) params.set('from', fromIso)
  if (toIso) params.set('to', toIso)

  const path = source === 'all' ? '/api/index' : `/api/index/${source}`
  const query = params.toString()
  const url = query ? `${path}?${query}` : path

  const res = await fetch(url)
  if (!res.ok) {
    throw new Error(`GET ${path} -> ${res.status}`)
  }
  return (await res.json()) as IndexCandles
}
