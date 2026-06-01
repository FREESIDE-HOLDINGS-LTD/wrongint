# AGENTS.md

## Ubiquitous language

Use these terms everywhere — code, comments, commits, conversation. Each maps to
the component / CSS that implements it.

- **Ticker** — top scrolling strip of index symbols + values. `Ticker.vue`.
- **Carousel** — scrolling row of **cards** below the ticker. `PostsCarousel.vue`.
- **Card** — one post in the carousel. `.card`.
- **Graph** — candlestick chart of an index. `IndexChart.vue`. Global graph (all
  sources) + one section graph per source.
- **Section** — one source's block: heroes, contenders, graph. `SourceSection.vue`.
- **Hero** — headline post above a graph. Two per section: **WIRED** (most
  contentious) and **TIRED** (calmest). `.contender.champion` (`champion` is the
  in-code spelling of hero).
- **Contender** — challenger post in the columns flanking a graph, ranked toward
  its hero (rank 1 = nearest). `.contender`.
