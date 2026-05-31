<script lang="ts">
import { defineComponent, markRaw, type PropType } from 'vue'
import uPlot from 'uplot'
import { indexSymbol, sourceColors, type IndexCandles } from '../api'
import { candlestickPlugin } from '../candlestick'
import DirArrows from './DirArrows.vue'

export default defineComponent({
  name: 'IndexChart',
  components: { DirArrows },
  props: {
    source: { type: String, required: true },
    candles: { type: Object as PropType<IndexCandles | null>, default: null },
    height: { type: Number, default: 260 },
    head: { type: Boolean, default: true },
  },
  computed: {
    symbol(): string {
      return indexSymbol(this.source)
    },
    colors(): { up: string; down: string } {
      return sourceColors(this.source)
    },
    valueColor(): string {
      if (this.latest == null || this.dir === 0) return '#5a6470'
      return this.dir > 0 ? this.colors.up : this.colors.down
    },
  },
  data(): { plot: uPlot | null; latest: number | null; dir: number; flashKey: number } {
    return { plot: null, latest: null, dir: 0, flashKey: 0 }
  },
  watch: {
    candles: {
      handler() {
        this.refresh()
      },
      deep: true,
    },
  },
  mounted() {
    this.refresh()
    window.addEventListener('resize', this.resize)
  },
  beforeUnmount() {
    window.removeEventListener('resize', this.resize)
    this.plot?.destroy()
  },
  methods: {
    // [xs, open, high, low, close] columns, x in unix seconds.
    toData(): uPlot.AlignedData {
      const xs: number[] = []
      const open: (number | null)[] = []
      const high: (number | null)[] = []
      const low: (number | null)[] = []
      const close: (number | null)[] = []
      for (const c of this.candles?.candles ?? []) {
        const hh = String(c.hour).padStart(2, '0')
        xs.push(Math.floor(new Date(`${c.date}T${hh}:00:00Z`).getTime() / 1000))
        open.push(c.open)
        high.push(c.high)
        low.push(c.low)
        close.push(c.close)
      }
      return [xs, open, high, low, close]
    },
    latestClose(): number | null {
      const candles = this.candles?.candles ?? []
      for (let i = candles.length - 1; i >= 0; i--) {
        if (candles[i].close != null) return candles[i].close
      }
      return null
    },
    options(width: number): uPlot.Options {
      const noLine = { paths: () => null, points: { show: false } }
      return {
        width,
        height: this.height,
        cursor: { drag: { x: true, y: false } },
        scales: { x: { time: true } },
        plugins: [
          candlestickPlugin({ up: this.colors.up, down: this.colors.down, wick: '#5a6470' }),
        ],
        series: [
          {},
          { label: 'open', ...noLine },
          { label: 'high', ...noLine },
          { label: 'low', ...noLine },
          { label: 'close', ...noLine },
        ],
        axes: [
          { stroke: '#5a6470', grid: { stroke: 'rgba(120,140,160,0.08)' } },
          { stroke: '#5a6470', grid: { stroke: 'rgba(120,140,160,0.08)' } },
        ],
      }
    },
    render() {
      const host = this.$refs.host as HTMLElement | undefined
      if (!host) return
      const data = this.toData()
      if (this.plot) {
        this.plot.setData(data)
      } else {
        this.plot = markRaw(new uPlot(this.options(host.clientWidth || 600), data, host))
      }
    },
    refresh() {
      this.render()
      const next = this.latestClose()
      if (next != null && this.latest != null) {
        this.dir = Math.sign(next - this.latest)
      }
      this.latest = next
      this.flashKey++
    },
    resize() {
      const host = this.$refs.host as HTMLElement | undefined
      if (this.plot && host) {
        this.plot.setSize({ width: host.clientWidth, height: this.height })
      }
    },
  },
})
</script>

<template>
  <div class="panel">
    <div v-if="head" class="chart-head">
      <span class="chart-title">{{ symbol }}</span>
      <span
        :key="flashKey"
        class="chart-value flash"
        :class="{ dead: latest == null }"
        :style="{ color: valueColor }"
      >
        <span class="chart-value-num" :class="{ blink: dir > 0 }">
          {{ latest == null ? '——' : latest.toFixed(0) }}
          <DirArrows :dir="dir" />
        </span>
      </span>
    </div>
    <div ref="host"></div>
  </div>
</template>
