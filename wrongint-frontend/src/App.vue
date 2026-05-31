<script lang="ts">
import { defineComponent } from 'vue'
import IndexChart from './components/IndexChart.vue'
import Ticker, { type TickerItem } from './components/Ticker.vue'
import PostsCarousel, { type CarouselPost } from './components/PostsCarousel.vue'
import SourceSection from './components/SourceSection.vue'
import {
  fetchIndexCandles,
  fetchSnapshot,
  indexSymbol,
  sourceColors,
  type IndexCandles,
  type Post,
} from './api'

const REFRESH_MS = 60_000

interface State {
  global: IndexCandles | null
  hackernews: IndexCandles | null
  lobsters: IndexCandles | null
  hnPosts: Post[]
  lobPosts: Post[]
  posts: CarouselPost[]
  error: string | null
  timer?: ReturnType<typeof setInterval>
}

// Last two non-null closes, newest first.
function recentCloses(candles: IndexCandles | null): number[] {
  const list = candles?.candles ?? []
  const closes: number[] = []
  for (let i = list.length - 1; i >= 0 && closes.length < 2; i--) {
    if (list[i].close != null) closes.push(list[i].close as number)
  }
  return closes
}

function latestOf(candles: IndexCandles | null): number | null {
  return recentCloses(candles)[0] ?? null
}

function dirOf(candles: IndexCandles | null): number {
  const closes = recentCloses(candles)
  return closes.length < 2 ? 0 : Math.sign(closes[0] - closes[1])
}

export default defineComponent({
  name: 'App',
  components: { IndexChart, Ticker, PostsCarousel, SourceSection },
  data(): State {
    return {
      global: null,
      hackernews: null,
      lobsters: null,
      hnPosts: [],
      lobPosts: [],
      posts: [],
      error: null,
      timer: undefined,
    }
  },
  computed: {
    ticker(): TickerItem[] {
      return [
        { source: 'all', value: latestOf(this.global), dir: dirOf(this.global) },
        { source: 'hackernews', value: latestOf(this.hackernews), dir: dirOf(this.hackernews) },
        { source: 'lobsters', value: latestOf(this.lobsters), dir: dirOf(this.lobsters) },
      ]
    },
    globalValue(): number | null {
      return latestOf(this.global)
    },
    globalDir(): number {
      return dirOf(this.global)
    },
    globalSymbol(): string {
      return indexSymbol('all')
    },
    globalColor(): string {
      if (this.globalValue == null || this.globalDir === 0) return '#5a6470'
      const c = sourceColors('all')
      return this.globalDir > 0 ? c.up : c.down
    },
  },
  mounted() {
    this.load()
    this.timer = setInterval(() => this.load(), REFRESH_MS)
  },
  beforeUnmount() {
    clearInterval(this.timer)
  },
  methods: {
    async load() {
      try {
        const [global, hackernews, lobsters, hnSnap, lobSnap] = await Promise.all([
          fetchIndexCandles('all'),
          fetchIndexCandles('hackernews'),
          fetchIndexCandles('lobsters'),
          fetchSnapshot('hackernews').catch(() => null),
          fetchSnapshot('lobsters').catch(() => null),
        ])
        this.global = global
        this.hackernews = hackernews
        this.lobsters = lobsters
        this.hnPosts = hnSnap?.posts ?? []
        this.lobPosts = lobSnap?.posts ?? []

        // Deterministically interleave the sources so they read mixed, not
        // clustered. Must be stable across refreshes: a random shuffle every
        // 60s reorders the carousel and makes the marquee jump. Same data ->
        // same order -> DOM unchanged -> animation keeps running.
        const lists = [hnSnap?.posts ?? [], lobSnap?.posts ?? []]
        const sources = [hnSnap?.source, lobSnap?.source]
        const posts: CarouselPost[] = []
        const max = Math.max(...lists.map((l) => l.length))
        for (let i = 0; i < max; i++) {
          for (let s = 0; s < lists.length; s++) {
            const p = lists[s][i]
            if (p) posts.push({ ...p, source: sources[s] as string })
          }
        }
        this.posts = posts

        this.error = null
      } catch (e) {
        this.error = e instanceof Error ? e.message : String(e)
      }
    },
  },
})
</script>

<template>
  <div class="scanlines"></div>
  <header class="topbar">
    <Ticker :items="ticker" />
    <PostsCarousel :posts="posts" />
  </header>

  <main>
    <h1>
      <span class="logo">FREESIDE GLOBAL OBSERVATION GROUP<span class="blink">_</span></span>
      <small>WE ARE MONITORING THE SITUATION</small>
    </h1>

    <p v-if="error" class="error blink">!! {{ error }}</p>

    <div class="global-hero">
      <span class="global-hero-num" :class="{ blink: globalDir > 0 }" :style="{ color: globalColor }">
        {{ globalValue == null ? '——' : globalValue.toFixed(0) }}
        <span v-if="globalDir !== 0" class="global-hero-arrow">{{ globalDir > 0 ? '▲' : '▼' }}</span>
      </span>
      <span class="global-hero-label">GLOBAL INTERNET DRAMA INDEX</span>
      <span class="global-hero-sym">{{ globalSymbol }}</span>
    </div>

    <IndexChart source="all" :candles="global" :height="340" :head="false" />

    <SourceSection source="hackernews" :candles="hackernews" :posts="hnPosts" />
    <SourceSection source="lobsters" :candles="lobsters" :posts="lobPosts" />
  </main>
</template>
