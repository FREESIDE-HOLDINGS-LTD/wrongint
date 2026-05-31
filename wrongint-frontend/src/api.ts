export type IndexSource = 'all' | 'hackernews' | 'lobsters'

// Ticker-style identifier for each index; used everywhere an index is named.
export const INDEX_SYMBOLS: Record<string, string> = {
  all: 'idx:global',
  hackernews: 'idx:hn',
  lobsters: 'idx:lob',
}

export function indexSymbol(source: string): string {
  return INDEX_SYMBOLS[source] ?? `idx:${source.toLowerCase()}`
}

// Full display names; used for section headers.
export const SOURCE_LABELS: Record<string, string> = {
  all: 'GLOBAL',
  hackernews: 'HACKER NEWS',
  lobsters: 'LOBSTE.RS',
}

export function sourceLabel(source: string): string {
  return SOURCE_LABELS[source] ?? source.toUpperCase()
}

// Per-index up/down colors. Up is the bright brand tone, down its dark variant.
export interface SourceColors {
  up: string
  down: string
}

export const SOURCE_COLORS: Record<string, SourceColors> = {
  all: { up: '#39ff14', down: '#ff2e63' },
  hackernews: { up: '#ffb454', down: '#9c5200' },
  lobsters: { up: '#c9aaff', down: '#5e35b1' },
}

export function sourceColors(source: string): SourceColors {
  return SOURCE_COLORS[source] ?? SOURCE_COLORS.all
}

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
