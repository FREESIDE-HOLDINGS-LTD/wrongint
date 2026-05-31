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
    mostContentious(): Post | null {
      return this.extreme((a, b) => b - a)
    },
    leastContentious(): Post | null {
      return this.extreme((a, b) => a - b)
    },
    range(): Range | null {
      return rangeOf(this.rated.map((p) => p.index))
    },
  },
  methods: {
    extreme(cmp: (a: number, b: number) => number): Post | null {
      const list = [...this.rated]
      list.sort((a, b) => cmp(a.index as number, b.index as number))
      return list[0] ?? null
    },
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(0)
    },
    accent(post: Post | null): string {
      return accent(heat(post?.index, this.range))
    },
    hot(post: Post | null): boolean {
      return isHot(heat(post?.index, this.range))
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
      <a
        v-for="item in [
          { emoji: '😡', label: 'WIRED', post: mostContentious },
          { emoji: '😴', label: 'TIRED', post: leastContentious },
        ]"
        :key="item.label"
        class="contender"
        :class="{ empty: !item.post, hot: hot(item.post) }"
        :style="{ '--accent': accent(item.post) }"
        :href="item.post?.comments_url"
        target="_blank"
        rel="noopener noreferrer"
      >
        <span class="contender-emoji">{{ item.emoji }}</span>
        <span class="contender-body">
          <span class="contender-label">{{ item.label }}</span>
          <span v-if="item.post" class="contender-title">{{ item.post.title }}</span>
          <span v-else class="contender-title muted">no data</span>
          <span v-if="item.post" class="contender-meta">
            <span class="contender-idx">idx {{ fmt(item.post.index) }}</span>
            <span class="contender-cs"
              >{{ item.post.comments }} comments / {{ item.post.score }} points</span
            >
          </span>
        </span>
      </a>
    </div>
    <IndexChart :source="source" :candles="candles" :height="320" :head="false" />
  </section>
</template>
