<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import IndexChart from './IndexChart.vue'
import DirArrows from './DirArrows.vue'
import PostCard from './PostCard.vue'
import {
  indexSymbol,
  sourceLabel,
  sourceUrl,
  sourceColors,
  type IndexCandles,
  type Post,
} from '../api'
import { rangeOf, heat, accent, isHot, emoji, type Range } from '../heat'

// Challengers stack in two vertical columns per side. Each column holds up to
// COLUMN_SIZE cards; with the champion that's PER_SIDE posts pulled per side.
const COLUMN_SIZE = 5
const PER_SIDE = COLUMN_SIZE * 2 + 1

export interface Contender {
  post: Post
  // 1 = strongest challenger (nearest the champion), growing outward along the
  // snake. Used to fade the ranks the farther they sit from the clash.
  rank: number
}

// Split a ranked challenger list (rank 1 first) into the two snaking columns.
// Inner column = ranks 1..COLUMN_SIZE, read top->bottom. Outer column =
// ranks COLUMN_SIZE+1..2*COLUMN_SIZE, read bottom->top (so reversed for DOM),
// continuing the queue's fold back up toward the champion.
function splitColumns(challengers: Contender[]): { inner: Contender[]; outer: Contender[] } {
  const inner = challengers.slice(0, COLUMN_SIZE)
  const outer = challengers.slice(COLUMN_SIZE, COLUMN_SIZE * 2).slice().reverse()
  return { inner, outer }
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
  components: { IndexChart, DirArrows, PostCard },
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
      return Math.min(PER_SIDE, Math.floor(this.byHeat.length / 2))
    },
    // WIRED champion: the single hottest (most contentious) post.
    wiredChampion(): Post | null {
      return this.byHeat[0] ?? null
    },
    // WIRED challengers, rank 1 first (strongest, nearest the champion).
    wiredChallengers(): Contender[] {
      return this.byHeat.slice(1, this.take).map((post, i) => ({ post, rank: i + 1 }))
    },
    // TIRED side: the coolest `take`, champion (calmest) first.
    tiredSide(): Post[] {
      return this.byHeat.slice(this.byHeat.length - this.take).reverse()
    },
    tiredChampion(): Post | null {
      return this.tiredSide[0] ?? null
    },
    tiredChallengers(): Contender[] {
      return this.tiredSide.slice(1).map((post, i) => ({ post, rank: i + 1 }))
    },
    wiredCols(): { inner: Contender[]; outer: Contender[] } {
      return splitColumns(this.wiredChallengers)
    },
    tiredCols(): { inner: Contender[]; outer: Contender[] } {
      return splitColumns(this.tiredChallengers)
    },
    range(): Range | null {
      return rangeOf(this.rated.map((p) => p.index))
    },
  },
  methods: {
    accent(post: Post | null): string {
      return accent(heat(post?.index, this.range))
    },
    hot(post: Post | null): boolean {
      return isHot(heat(post?.index, this.range))
    },
    emoji(post: Post | null): string {
      return emoji(heat(post?.index, this.range))
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
          <DirArrows :dir="dir" />
        </span>
      </span>
    </h2>
    <!-- Battle line: challenger columns flank the central arena (champions row
         stacked on top of the chart) and run its full height. -->
    <div class="battle">
      <!-- WIRED flank: outer column toward the page edge, inner column nearest
           the arena/champion. -->
      <div class="flank flank--wired">
        <!-- Outer column: ranks high->low top to bottom, flow points down to
             the fold at the bottom. -->
        <div class="column column--outer">
          <template v-for="(c, i) in wiredCols.outer" :key="c.post.id">
            <PostCard
              kind="contender"
              sm
              :post="c.post"
              :label="'#' + c.rank"
              :accent="accent(c.post)"
              :hot="hot(c.post)"
              :emoji="emoji(c.post)"
              :style="{ opacity: dim(c.rank) }"
            />
            <span v-if="i < wiredCols.outer.length - 1" class="advance advance--down" aria-hidden="true">➤</span>
          </template>
        </div>
        <!-- The fold: outer column hands off to the inner column at the bottom. -->
        <span v-if="wiredCols.outer.length" class="advance advance--right advance--fold" aria-hidden="true">➤</span>
        <!-- Inner column: ranks low->high top to bottom, flow points up into
             the champion. -->
        <div class="column column--inner">
          <template v-for="(c, i) in wiredCols.inner" :key="c.post.id">
            <span v-if="i > 0" class="advance advance--up" aria-hidden="true">➤</span>
            <PostCard
              kind="contender"
              sm
              :post="c.post"
              :label="'#' + c.rank"
              :accent="accent(c.post)"
              :hot="hot(c.post)"
              :emoji="emoji(c.post)"
              :style="{ opacity: dim(c.rank) }"
            />
          </template>
        </div>
        <!-- Inner column's strongest contender charges into the champion. -->
        <span v-if="wiredCols.inner.length" class="advance advance--right advance--hero" aria-hidden="true">➤</span>
      </div>

      <!-- Central arena: the two champions clash on top, the chart below. Its
           width matches the chart; the flanks fill the outer margins. -->
      <div class="arena">
        <div class="champions">
          <PostCard
            v-if="wiredChampion"
            kind="champion"
            label="WIRED"
            :post="wiredChampion"
            :accent="accent(wiredChampion)"
            :hot="hot(wiredChampion)"
            :emoji="emoji(wiredChampion)"
          />

          <!-- The clash: the two champions meeting in the middle, lightning
               arcing between them, sparks flying. -->
          <div v-if="wiredChampion && tiredChampion" class="clash" aria-hidden="true">
            <span class="spark spark--1">✦</span>
            <span class="spark spark--2">✧</span>
            <span class="spark spark--3">✦</span>
            <span class="spark spark--4">✧</span>
            <span class="bolt bolt--1">⚡</span>
            <span class="bolt bolt--2">⚡</span>
            <span class="clash-core">⚡</span>
          </div>

          <PostCard
            v-if="tiredChampion"
            kind="champion"
            label="TIRED"
            :post="tiredChampion"
            :accent="accent(tiredChampion)"
            :hot="hot(tiredChampion)"
            :emoji="emoji(tiredChampion)"
          />
        </div>
        <IndexChart :source="source" :candles="candles" :height="320" :head="false" />
      </div>

      <!-- TIRED flank: inner column nearest the arena, outer column toward the
           page edge (mirror of WIRED). -->
      <div class="flank flank--tired">
        <!-- Inner column's strongest contender charges into the champion. -->
        <span v-if="tiredCols.inner.length" class="advance advance--left advance--hero" aria-hidden="true">➤</span>
        <div class="column column--inner">
          <template v-for="(c, i) in tiredCols.inner" :key="c.post.id">
            <span v-if="i > 0" class="advance advance--up" aria-hidden="true">➤</span>
            <PostCard
              kind="contender"
              sm
              :post="c.post"
              :label="'#' + c.rank"
              :accent="accent(c.post)"
              :hot="hot(c.post)"
              :emoji="emoji(c.post)"
              :style="{ opacity: dim(c.rank) }"
            />
          </template>
        </div>
        <!-- The fold: outer column hands off to the inner column at the bottom. -->
        <span v-if="tiredCols.outer.length" class="advance advance--left advance--fold" aria-hidden="true">➤</span>
        <div class="column column--outer">
          <template v-for="(c, i) in tiredCols.outer" :key="c.post.id">
            <PostCard
              kind="contender"
              sm
              :post="c.post"
              :label="'#' + c.rank"
              :accent="accent(c.post)"
              :hot="hot(c.post)"
              :emoji="emoji(c.post)"
              :style="{ opacity: dim(c.rank) }"
            />
            <span v-if="i < tiredCols.outer.length - 1" class="advance advance--down" aria-hidden="true">➤</span>
          </template>
        </div>
      </div>
    </div>
  </section>
</template>
