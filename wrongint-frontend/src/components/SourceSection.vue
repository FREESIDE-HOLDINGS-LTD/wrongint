<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import IndexChart from './IndexChart.vue'
import {
  indexSymbol,
  sourceLabel,
  sourceUrl,
  sourceColors,
  type IndexCandles,
  type Post,
} from '../api'
import { rangeOf, heat, accent, isHot, type Range } from '../heat'

// Posts shown per side of the clash (champion + follow-up challengers).
const SIDE_SIZE = 3

export interface Contender {
  post: Post
  champion: boolean
  // 0 at the center (the clash), growing outward — used to fade the ranks.
  rank: number
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

export default defineComponent({
  name: 'SourceSection',
  components: { IndexChart },
  props: {
    source: { type: String, required: true },
    candles: { type: Object as PropType<IndexCandles | null>, default: null },
    posts: { type: Array as PropType<Post[]>, default: () => [] },
  },
  computed: {
    symbol(): string {
      return indexSymbol(this.source)
    },
    label(): string {
      return sourceLabel(this.source)
    },
    url(): string | null {
      return sourceUrl(this.source)
    },
    headColor(): string {
      return sourceColors(this.source).up
    },
    latest(): number | null {
      return recentCloses(this.candles)[0] ?? null
    },
    dir(): number {
      const closes = recentCloses(this.candles)
      return closes.length < 2 ? 0 : Math.sign(closes[0] - closes[1])
    },
    readoutColor(): string {
      if (this.latest == null || this.dir === 0) return '#5a6470'
      const c = sourceColors(this.source)
      return this.dir > 0 ? c.up : c.down
    },
    rated(): Post[] {
      return this.posts.filter((p) => p.index != null)
    },
    // Hottest first.
    byHeat(): Post[] {
      return [...this.rated].sort((a, b) => (b.index as number) - (a.index as number))
    },
    // How many to take per side without the two sides overlapping.
    take(): number {
      return Math.min(SIDE_SIZE, Math.floor(this.byHeat.length / 2))
    },
    // WIRED team. Ordered left -> center: weakest challenger first, champion
    // (most contentious) last so it sits against the clash in the middle.
    wired(): Contender[] {
      const top = this.byHeat.slice(0, this.take) // hottest first
      return top
        .map((post, i) => ({ post, champion: i === 0, rank: i }))
        .reverse()
    },
    // TIRED team. Ordered center -> right: champion (calmest) first, against
    // the clash; weaker challengers trail off to the right.
    tired(): Contender[] {
      const n = this.byHeat.length
      const bottom = this.byHeat.slice(n - this.take) // ...coolest last
      return bottom
        .map((post, i) => ({ post, champion: i === bottom.length - 1, rank: i }))
        .reverse() // coolest (champion) first
        .map((c, i) => ({ ...c, rank: i }))
    },
    wiredChampion(): Contender | null {
      return this.wired.find((c) => c.champion) ?? null
    },
    wiredChallengers(): Contender[] {
      return this.wired.filter((c) => !c.champion)
    },
    tiredChampion(): Contender | null {
      return this.tired.find((c) => c.champion) ?? null
    },
    tiredChallengers(): Contender[] {
      return this.tired.filter((c) => !c.champion)
    },
    range(): Range | null {
      return rangeOf(this.rated.map((p) => p.index))
    },
  },
  methods: {
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(0)
    },
    accent(post: Post | null): string {
      return accent(heat(post?.index, this.range))
    },
    hot(post: Post | null): boolean {
      return isHot(heat(post?.index, this.range))
    },
    // Challengers further from the clash are dimmer.
    dim(rank: number): number {
      return Math.max(0.45, 1 - rank * 0.22)
    },
  },
})
</script>

<template>
  <section class="source-section">
    <h2 class="section-head" :style="{ color: headColor, borderColor: headColor }">
      <a
        v-if="url"
        class="section-head-name"
        :href="url"
        target="_blank"
        rel="noopener noreferrer"
      >
        <span class="section-head-mark">▌</span>{{ label }}
      </a>
      <span v-else class="section-head-name"><span class="section-head-mark">▌</span>{{ label }}</span>
      <span class="section-readout" :style="{ color: readoutColor }">
        <span class="section-readout-sym">{{ symbol }}</span>
        <span class="section-readout-num" :class="{ blink: dir > 0 }">
          <span class="section-readout-val">{{ latest == null ? '——' : latest.toFixed(0) }}</span>
          <span v-if="dir !== 0" class="section-readout-arrow">{{ dir > 0 ? '▲' : '▼' }}</span>
        </span>
      </span>
    </h2>
    <div class="contention">
      <!-- WIRED team: challengers fill the outer margin, champion pinned to the
           chart edge (inner) so its outer edge lines up with the chart. -->
      <div class="team team--wired">
        <div class="ranks ranks--wired">
          <template v-for="(c, i) in wiredChallengers" :key="c.post.id">
            <a
              class="contender"
              :class="{ hot: hot(c.post) }"
              :style="{ '--accent': accent(c.post), opacity: dim(c.rank) }"
              :href="c.post.comments_url"
              target="_blank"
              rel="noopener noreferrer"
            >
              <span class="contender-body">
                <span class="contender-title">{{ c.post.title }}</span>
                <span class="contender-meta">
                  <span class="contender-idx">idx {{ fmt(c.post.index) }}</span>
                  <span class="contender-cs">{{ c.post.comments }} comments / {{ c.post.score }} points</span>
                </span>
              </span>
            </a>
            <span v-if="i < wiredChallengers.length - 1" class="advance advance--right" aria-hidden="true">➤</span>
          </template>
          <span v-if="wiredChallengers.length" class="advance advance--right" aria-hidden="true">➤</span>
        </div>
        <a
          v-if="wiredChampion"
          class="contender champion"
          :class="{ hot: hot(wiredChampion.post) }"
          :style="{ '--accent': accent(wiredChampion.post) }"
          :href="wiredChampion.post.comments_url"
          target="_blank"
          rel="noopener noreferrer"
        >
          <span class="contender-emoji">😡</span>
          <span class="contender-body">
            <span class="contender-label">WIRED</span>
            <span class="contender-title">{{ wiredChampion.post.title }}</span>
            <span class="contender-meta">
              <span class="contender-idx">idx {{ fmt(wiredChampion.post.index) }}</span>
              <span class="contender-cs">{{ wiredChampion.post.comments }} comments / {{ wiredChampion.post.score }} points</span>
            </span>
          </span>
        </a>
      </div>

      <!-- The clash: the two champions meeting in the middle, lightning
           arcing between them, sparks flying. -->
      <div v-if="wired.length && tired.length" class="clash" aria-hidden="true">
        <span class="spark spark--1">✦</span>
        <span class="spark spark--2">✧</span>
        <span class="spark spark--3">✦</span>
        <span class="spark spark--4">✧</span>
        <span class="bolt bolt--1">⚡</span>
        <span class="bolt bolt--2">⚡</span>
        <span class="clash-core">⚡</span>
      </div>

      <!-- TIRED team: champion pinned to chart edge (inner), challengers trail
           out to the page edge. -->
      <div class="team team--tired">
        <a
          v-if="tiredChampion"
          class="contender champion"
          :class="{ hot: hot(tiredChampion.post) }"
          :style="{ '--accent': accent(tiredChampion.post) }"
          :href="tiredChampion.post.comments_url"
          target="_blank"
          rel="noopener noreferrer"
        >
          <span class="contender-emoji">😴</span>
          <span class="contender-body">
            <span class="contender-label">TIRED</span>
            <span class="contender-title">{{ tiredChampion.post.title }}</span>
            <span class="contender-meta">
              <span class="contender-idx">idx {{ fmt(tiredChampion.post.index) }}</span>
              <span class="contender-cs">{{ tiredChampion.post.comments }} comments / {{ tiredChampion.post.score }} points</span>
            </span>
          </span>
        </a>
        <div class="ranks ranks--tired">
          <span v-if="tiredChallengers.length" class="advance advance--left" aria-hidden="true">➤</span>
          <template v-for="(c, i) in tiredChallengers" :key="c.post.id">
            <a
              class="contender"
              :class="{ hot: hot(c.post) }"
              :style="{ '--accent': accent(c.post), opacity: dim(c.rank) }"
              :href="c.post.comments_url"
              target="_blank"
              rel="noopener noreferrer"
            >
              <span class="contender-body">
                <span class="contender-title">{{ c.post.title }}</span>
                <span class="contender-meta">
                  <span class="contender-idx">idx {{ fmt(c.post.index) }}</span>
                  <span class="contender-cs">{{ c.post.comments }} comments / {{ c.post.score }} points</span>
                </span>
              </span>
            </a>
            <span v-if="i < tiredChallengers.length - 1" class="advance advance--left" aria-hidden="true">➤</span>
          </template>
        </div>
      </div>
    </div>
    <IndexChart :source="source" :candles="candles" :height="320" :head="false" />
  </section>
</template>
