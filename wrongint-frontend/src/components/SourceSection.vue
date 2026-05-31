<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import IndexChart from './IndexChart.vue'
import type { IndexCandles, Post } from '../api'
import { rangeOf, heat, accent, isHot, type Range } from '../heat'

export default defineComponent({
  name: 'SourceSection',
  components: { IndexChart },
  props: {
    title: { type: String, required: true },
    candles: { type: Object as PropType<IndexCandles | null>, default: null },
    posts: { type: Array as PropType<Post[]>, default: () => [] },
    color: { type: String, default: '#39ff14' },
  },
  computed: {
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
    <h2 class="section-head" :style="{ color, borderColor: color }">
      <span class="section-head-mark">▌</span>{{ title }}
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
    <IndexChart :title="title" :candles="candles" :color="color" :height="320" />
  </section>
</template>
