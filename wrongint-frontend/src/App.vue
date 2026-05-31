<script lang="ts">
import { defineComponent } from 'vue'
import IndexChart from './components/IndexChart.vue'
import Ticker, { type TickerItem } from './components/Ticker.vue'
import PostsCarousel, { type CarouselPost } from './components/PostsCarousel.vue'
import SourceSection from './components/SourceSection.vue'
import { fetchIndexCandles, fetchSnapshot, type IndexCandles, type Post } from './api'

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

function latestOf(candles: IndexCandles | null): number | null {
  const list = candles?.candles ?? []
  for (let i = list.length - 1; i >= 0; i--) {
    if (list[i].close != null) return list[i].close
  }
  return null
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
        { label: 'WRONGINT::GLOBAL', value: latestOf(this.global) },
        { label: 'HACKERNEWS', value: latestOf(this.hackernews) },
        { label: 'LOBSTERS', value: latestOf(this.lobsters) },
      ]
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

        const posts: CarouselPost[] = []
        for (const snap of [hnSnap, lobSnap]) {
          if (!snap) continue
          for (const p of snap.posts) posts.push({ ...p, source: snap.source })
        }
        posts.sort((a, b) => (b.index ?? -1) - (a.index ?? -1))
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
  <Ticker :items="ticker" />
  <PostsCarousel :posts="posts" />

  <main>
    <h1>
      <span class="logo">FREESIDE OBSERVATION GROUP</span>
      <span class="blink">_</span>
      <small>WE ARE MONITORING THE SITUATION</small>
    </h1>

    <p v-if="error" class="error blink">!! {{ error }}</p>

    <IndexChart title="GLOBAL" :candles="global" :height="340" color="#39ff14" />

    <SourceSection title="HACKER NEWS" :candles="hackernews" :posts="hnPosts" color="#ff9f1c" />
    <SourceSection title="LOBSTE.RS" :candles="lobsters" :posts="lobPosts" color="#b388ff" />
  </main>
</template>
