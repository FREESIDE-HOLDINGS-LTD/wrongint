<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import { Vue3Marquee } from 'vue3-marquee'
import { indexSymbol, sourceColors } from '../api'

export interface TickerItem {
  source: string
  value: number | null
  dir: number
}

export default defineComponent({
  name: 'Ticker',
  components: { Vue3Marquee },
  props: {
    items: { type: Array as PropType<TickerItem[]>, default: () => [] },
  },
  methods: {
    indexSymbol,
    fmt(v: number | null): string {
      return v == null ? '——' : v.toFixed(0)
    },
    color(it: TickerItem): string {
      if (it.value == null || it.dir === 0) return '#5a6470'
      const c = sourceColors(it.source)
      return it.dir > 0 ? c.up : c.down
    },
    arrow(dir: number): string {
      return dir > 0 ? '▲' : dir < 0 ? '▼' : ''
    },
  },
})
</script>

<template>
  <Vue3Marquee class="ticker" :duration="2" :clone="true" :pause-on-hover="true">
    <span v-for="it in items" :key="it.source" class="tick">
      <span class="tick-label">{{ indexSymbol(it.source) }}</span>
      <span
        class="tick-value"
        :class="{ dead: it.value == null, blink: it.dir > 0 }"
        :style="{ color: color(it) }"
      >
        {{ fmt(it.value) }}
        <span v-if="it.dir !== 0" class="tick-arrow">{{ arrow(it.dir) }}</span>
      </span>
    </span>
  </Vue3Marquee>
</template>
