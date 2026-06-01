<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import { Vue3Marquee } from 'vue3-marquee'
import DirArrows from './DirArrows.vue'
import { indexSymbol, sourceColors } from '../api'
import { slow, resume } from '../marquee-hover'

export interface TickerItem {
  source: string
  value: number | null
  dir: number
}

export default defineComponent({
  name: 'Ticker',
  components: { Vue3Marquee, DirArrows },
  props: {
    items: { type: Array as PropType<TickerItem[]>, default: () => [] },
  },
  methods: {
    slow,
    resume,
    indexSymbol,
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(0)
    },
    color(it: TickerItem): string {
      if (it.value == null || it.dir === 0) return '#5a6470'
      const c = sourceColors(it.source)
      return it.dir > 0 ? c.up : c.down
    },
  },
})
</script>

<template>
  <Vue3Marquee
    class="ticker"
    :duration="2"
    :clone="true"
    @mouseenter="slow"
    @mouseleave="resume"
  >
    <span v-for="it in items" :key="it.source" class="tick">
      <span class="tick-label">{{ indexSymbol(it.source) }}</span>
      <span
        class="tick-value"
        :class="{ dead: it.value == null, blink: it.dir > 0 }"
        :style="{ color: color(it) }"
      >
        {{ fmt(it.value) }}
        <DirArrows :dir="it.dir" />
      </span>
    </span>
  </Vue3Marquee>
</template>
