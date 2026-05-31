import uPlot from 'uplot'

export interface CandleColors {
  up: string
  down: string
  wick: string
}

// Draws OHLC candlesticks over the chart. Expects data laid out as
// [xs, open, high, low, close]; the four value series should render no line of
// their own (paths: () => null) so this hook owns the visuals.
export function candlestickPlugin(colors: CandleColors): uPlot.Plugin {
  return {
    hooks: {
      draw: (u: uPlot) => {
        const xs = u.data[0]
        const open = u.data[1]
        const high = u.data[2]
        const low = u.data[3]
        const close = u.data[4]
        if (!open || !high || !low || !close) return

        const { ctx } = u
        const n = xs.length

        let colWidth = 20
        if (n > 1) {
          colWidth = Math.abs(u.valToPos(xs[1], 'x', true) - u.valToPos(xs[0], 'x', true))
        }
        const bodyW = Math.max(Math.min(colWidth * 0.6, 16), 1)

        ctx.save()
        ctx.lineWidth = 1
        for (let i = 0; i < n; i++) {
          const o = open[i]
          const h = high[i]
          const l = low[i]
          const c = close[i]
          if (o == null || h == null || l == null || c == null) continue

          const x = Math.round(u.valToPos(xs[i], 'x', true))
          const yOpen = Math.round(u.valToPos(o, 'y', true))
          const yClose = Math.round(u.valToPos(c, 'y', true))
          const yHigh = Math.round(u.valToPos(h, 'y', true))
          const yLow = Math.round(u.valToPos(l, 'y', true))
          const color = c >= o ? colors.up : colors.down

          ctx.strokeStyle = colors.wick
          ctx.beginPath()
          ctx.moveTo(x + 0.5, yHigh)
          ctx.lineTo(x + 0.5, yLow)
          ctx.stroke()

          ctx.fillStyle = color
          const top = Math.min(yOpen, yClose)
          const height = Math.max(Math.abs(yClose - yOpen), 2)
          ctx.fillRect(Math.round(x - bodyW / 2), top, Math.round(bodyW), height)

          // Always mark the close so a lone or flat (doji) candle stays visible.
          ctx.beginPath()
          ctx.arc(x + 0.5, yClose, 2.5, 0, Math.PI * 2)
          ctx.fill()
        }
        ctx.restore()
      },
    },
  }
}
