---
name: d3-data-visualization
description: Patterns, idioms, and decision framework for building data visualizations with D3.js v7, including data join mechanics, scales, axes, transitions, React integration strategies, responsive SVG, brush/zoom, force simulation, and when to reach for a higher-level library instead.
domain: frontend
category: visualization
tags: [D3.js, data-visualization, SVG, charts, scales, axes, React, transitions, brush, zoom, force-simulation]
triggers: [d3, d3.js, data visualization, svg chart, scatter plot, bar chart, line chart, scales, axes, data join, enter exit update, force simulation, brush zoom, recharts vs d3, visx, observable plot]
---

# D3.js Data Visualization

## What D3 actually is

D3 (Data-Driven Documents) is not a charting library. It is a collection of low-level modules for binding data to the DOM and transforming it into visual marks. You compose those primitives yourself. This is its strength and its cost: maximum expressiveness, high implementation effort. Use it when the chart type does not exist in a higher-level library, when you need full control over interaction and animation, or when bundle constraints make a slimmer custom build worthwhile.

Current stable: **v7.9.0** (as of 2025). All patterns here target v7.

---

## Data join — the core mental model

Every D3 rendering operation is a join between a data array and a set of DOM elements. Understanding this is non-negotiable.

```text
data array  ─┐
              ├─ JOIN ─► update selection  (data matched existing element)
DOM elements ─┘       ► enter selection   (data has no element yet)
                       ► exit selection    (element has no matching data)
```

The key function passed to `.data()` controls how matching happens. Without it, D3 matches by index, which breaks when the array is reordered or filtered.

```javascript
// index-based join (fragile for dynamic data)
selection.data(data)

// key-based join (robust)
selection.data(data, d => d.id)
```

Data bound via `.data()` is stored on the DOM node as `.__data__`. Bound via `.datum()` it skips the join entirely — no enter/exit — useful for single-element containers like a root `<svg>`.

---

## .join() — the v7 idiom

Before v5, you wrote the general update pattern explicitly:

```javascript
// old pattern — still works, occasionally necessary
const circle = svg.selectAll("circle").data(data, d => d.id);
circle.exit().remove();
const entered = circle.enter().append("circle").attr("r", 0);
circle.merge(entered).attr("cx", d => x(d.value));
```

In v7 the canonical form is `.join()`:

```javascript
// simple string form — handles enter/exit automatically
svg.selectAll("circle")
  .data(data, d => d.id)
  .join("circle")
    .attr("cx", d => x(d.value))
    .attr("cy", d => y(d.category))
    .attr("r", 5);
```

Attributes set after `.join()` apply to both entering and updating elements. When you need different treatment per lifecycle phase, use callbacks:

```javascript
svg.selectAll("circle")
  .data(data, d => d.id)
  .join(
    enter => enter.append("circle")
      .attr("r", 0)
      .attr("fill", "steelblue")
      .call(el => el.transition(t).attr("r", 5)),   // fade in
    update => update
      .call(el => el.transition(t).attr("cx", d => x(d.value))),
    exit => exit
      .call(el => el.transition(t).attr("r", 0).remove())
  )
  .attr("cy", d => y(d.category));
```

The exit callback's return value is ignored — only enter and update are merged.

---

## Scales

Scales map from a data domain to a visual range. They are pure functions; they do not render anything.

### Continuous scales

```javascript
const x = d3.scaleLinear()
  .domain([0, d3.max(data, d => d.value)])
  .range([0, innerWidth]);

const color = d3.scaleSequential(d3.interpolateViridis)
  .domain([0, 100]);
```

- `scaleLinear` — numeric, proportional
- `scaleSqrt` / `scalePow` — useful for bubble chart radii (area ∝ data, so radius ∝ √data)
- `scaleLog` — skewed distributions
- `scaleTime` — Date objects on the domain

### Ordinal / band scales

```javascript
const x = d3.scaleBand()
  .domain(data.map(d => d.name))
  .range([0, innerWidth])
  .padding(0.2);   // proportional gap between bands

// bar width
x.bandwidth()

// position of category "Alpha"
x("Alpha")
```

`scalePoint` is like `scaleBand` but places points at the center of each step — good for dot plots and parallel coordinates.

### Color scales

```javascript
// categorical
const color = d3.scaleOrdinal(d3.schemeTableau10).domain(categories);

// sequential
const heat = d3.scaleSequential([0, maxVal], d3.interpolateYlOrRd);

// diverging
const div = d3.scaleDiverging([-1, 0, 1], d3.interpolateRdBu);
```

---

## Axes and the margins convention

D3 axes render tick marks, labels, and the axis line into a `<g>` element. They read from a scale you already defined.

```javascript
const axisBottom = d3.axisBottom(x)
  .ticks(6)
  .tickFormat(d3.format(".2s"));  // "1.2k", "3.4M"

svg.append("g")
  .attr("transform", `translate(0, ${innerHeight})`)
  .call(axisBottom);
```

**The margins convention** is the standard way to handle padding in D3. Define an outer SVG size and an inner drawing area:

```javascript
const margin = { top: 20, right: 30, bottom: 40, left: 50 };
const width  = 800;
const height = 500;
const innerWidth  = width  - margin.left - margin.right;
const innerHeight = height - margin.top  - margin.bottom;

const svg = d3.select("#chart")
  .append("svg")
    .attr("width",  width)
    .attr("height", height)
  .append("g")
    .attr("transform", `translate(${margin.left},${margin.top})`);
```

All subsequent drawing happens in `svg` (the inner `<g>`). Axes, grid lines, and data marks use `innerWidth` / `innerHeight` for their ranges and positions.

---

## Transitions

D3 transitions interpolate attributes and styles between states. They are queued per element and can be named to allow coordinated multi-step animations.

```javascript
const t = d3.transition()
  .duration(600)
  .ease(d3.easeCubicOut);

// apply to a selection
bars.transition(t)
  .attr("y",      d => y(d.value))
  .attr("height", d => innerHeight - y(d.value));
```

Rules for transitions:

- Chain `.transition()` on a selection, not before `.data()`.
- Reuse a named transition (`d3.transition("update")`) across selections so that they synchronize.
- Don't mix transitions and immediate attribute sets on the same element in the same tick — the transition wins, then overwrites.
- For enter animations, set the "from" state before the transition, then set the "to" state inside it.

```javascript
enter => enter.append("rect")
  .attr("y", innerHeight)          // start below axis
  .attr("height", 0)               // zero height
  .call(el => el.transition(t)
    .attr("y",      d => y(d.value))
    .attr("height", d => innerHeight - y(d.value)))
```

---

## Responsive SVG

Two approaches:

**viewBox approach** — scales the entire SVG proportionally, no JS needed for resize:

```javascript
d3.select("#chart").append("svg")
  .attr("viewBox", `0 0 ${width} ${height}`)
  .attr("preserveAspectRatio", "xMinYMin meet")
  .style("width",  "100%")
  .style("height", "auto");
```

Limitation: tick labels and stroke widths scale with the SVG, which can look wrong at extreme sizes.

**ResizeObserver approach** — re-renders with new dimensions, preserves absolute font/stroke sizes:

```javascript
const ro = new ResizeObserver(entries => {
  const { width } = entries[0].contentRect;
  render(width);
});
ro.observe(document.querySelector("#chart"));

function render(containerWidth) {
  const innerWidth = containerWidth - margin.left - margin.right;
  x.range([0, innerWidth]);
  // rebind data and redraw
}
```

Use viewBox for simple static charts. Use ResizeObserver when you also want adaptive tick counts, conditional label display, or different layouts at different breakpoints.

---

## Brush and zoom

**Brush** selects a rectangular region and fires events with the pixel extent, which you then invert through your scale.

```javascript
const brush = d3.brushX()
  .extent([[0, 0], [innerWidth, innerHeight]])
  .on("end", brushed);

svg.append("g").attr("class", "brush").call(brush);

function brushed({ selection }) {
  if (!selection) return;
  const [x0, x1] = selection.map(x.invert);
  // filter data to [x0, x1]
}
```

**Zoom** applies a transform (translate + scale) to a `<g>` and fires events with the current `d3.ZoomTransform`.

```javascript
const zoom = d3.zoom()
  .scaleExtent([1, 8])
  .on("zoom", zoomed);

svg.call(zoom);

function zoomed({ transform }) {
  const newX = transform.rescaleX(x);   // rescaled copy of the original scale
  gX.call(axisBottom.scale(newX));
  circles.attr("cx", d => newX(d.date));
}
```

The pattern `transform.rescaleX(originalScale)` creates a new scale that accounts for the zoom level without mutating the original — keep the original around for brush inversion.

Brush + zoom combined (focus+context): use brush on a small overview chart to set the domain of a zoomed main chart. This is the standard "crossfilter" pattern.

---

## Force simulation

D3 force simulation iterates a physics engine over a set of nodes (and optionally links) until it reaches a low-energy state. It does not render anything; you read node positions after each tick.

```javascript
const simulation = d3.forceSimulation(nodes)
  .force("link",   d3.forceLink(links).id(d => d.id).distance(60))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("center", d3.forceCenter(innerWidth / 2, innerHeight / 2))
  .force("collide", d3.forceCollide(d => d.radius + 2));

simulation.on("tick", () => {
  link.attr("x1", d => d.source.x)
      .attr("y1", d => d.source.y)
      .attr("x2", d => d.target.x)
      .attr("y2", d => d.target.y);

  node.attr("cx", d => d.x)
      .attr("cy", d => d.y);
});
```

Key forces:

- `forceLink` — spring-like edges, set `.distance()` and `.strength()`
- `forceManyBody` — negative strength = repulsion (graph layout); positive = gravity-like clustering
- `forceCenter` — pulls all nodes toward a point (weak gravity)
- `forceCollide` — prevents node overlap; supply radius per node
- `forceX` / `forceY` — pulls nodes toward a target x or y, useful for grouped beeswarm layouts

For static layouts (beeswarm, bubble chart) call `simulation.stop()` and run the engine manually:

```javascript
simulation.stop();
for (let i = 0; i < 300; i++) simulation.tick();
// now read node.x / node.y and render once
```

---

## React + D3 integration

React and D3 both want to own the DOM. Three strategies, in order of preference for production:

### 1. D3 for math, React for DOM (recommended)

D3 computes scales, paths, and positions. React renders the SVG elements via JSX. No D3 DOM manipulation at all.

```jsx
function BarChart({ data }) {
  const margin = { top: 20, right: 20, bottom: 30, left: 40 };
  const width = 600, height = 300;
  const innerWidth  = width  - margin.left - margin.right;
  const innerHeight = height - margin.top  - margin.bottom;

  const x = d3.scaleBand()
    .domain(data.map(d => d.name))
    .range([0, innerWidth])
    .padding(0.2);

  const y = d3.scaleLinear()
    .domain([0, d3.max(data, d => d.value)])
    .range([innerHeight, 0]);

  return (
    <svg width={width} height={height}>
      <g transform={`translate(${margin.left},${margin.top})`}>
        {data.map(d => (
          <rect
            key={d.name}
            x={x(d.name)}
            y={y(d.value)}
            width={x.bandwidth()}
            height={innerHeight - y(d.value)}
            fill="steelblue"
          />
        ))}
      </g>
    </svg>
  );
}
```

Axes are the main awkward spot — use a `useEffect` with a ref:

```jsx
const xAxisRef = useRef();
useEffect(() => {
  d3.select(xAxisRef.current).call(d3.axisBottom(x));
}, [x]);
// <g ref={xAxisRef} transform={`translate(0,${innerHeight})`} />
```

### 2. D3 owns a DOM node via useRef

Hand D3 a container element and let it render directly. Appropriate for highly animated charts or when porting an existing D3 example verbatim.

```jsx
const ref = useRef();
useEffect(() => {
  const svg = d3.select(ref.current);
  // full D3 rendering logic here
  return () => svg.selectAll("*").remove(); // cleanup
}, [data]);

return <div ref={ref} />;
```

Downside: React has no visibility into what D3 placed inside the node. React DevTools and SSR won't see those elements.

### 3. Visx (Airbnb)

Visx wraps D3 logic in React components, giving you D3-level power with React-native composability and TypeScript types. Choose Visx when the team is React-first and wants type safety throughout.

```jsx
import { Bar } from "@visx/shape";
import { scaleBand, scaleLinear } from "@visx/scale";
```

---

## When to use D3 vs alternatives

| Need | Reach for |
| ------ | ----------- |
| Custom, bespoke, or novel chart type | D3 |
| Full control over interaction (brush, custom tooltips, linked views) | D3 |
| React app, standard chart types (line, bar, pie, area), fast delivery | Recharts |
| React app, custom charts with type safety, low-level but React-native | Visx |
| Exploratory analysis, readable code, rapid prototyping | Observable Plot |
| Static images / server-side rendering without a browser | D3 (jsdom or node-canvas) |
| Very large datasets (>100k points) with GPU rendering | Deck.gl or regl |

**Recharts** — opinionated, excellent defaults, component-based API, good documentation. Customization runs out at medium complexity. Bundle: ~150 KB gzipped with React.

**Visx** — Airbnb's primitives library. Modular (tree-shakeable), low-level like D3 but React-native. Closer to D3 in effort; closer to React in integration. Good for shared component systems.

**Observable Plot** — created by Mike Bostock (D3's author) for exploratory visualization. Declarative mark-based API inspired by Vega-Lite. Excellent for notebooks and dashboards. Less suited for custom interactions.

---

## Common mistakes

**Mutating scale domains inside a zoom handler** — always use `transform.rescaleX(originalScale)` to produce a derived scale, not `x.domain(newDomain)`. Mutating the original breaks brush inversion and axis ticks.

**Recreating the SVG on every render in React** — append once; update via data join. Guard with a ref flag or check for existing `<svg>` children before appending.

**Missing the key function on data joins** — when data is reordered or filtered, index-based joins animate elements to wrong positions. Always pass a key when data identity matters.

**Running force simulation without stopping it on unmount** — in React, the simulation's `tick` callback holds references to DOM nodes. Call `simulation.stop()` in the `useEffect` cleanup.

**Hardcoding pixel dimensions** — always derive `innerWidth` / `innerHeight` from a ResizeObserver or a container measurement. Charts that look fine at 1440px break at 375px.

---

## Typical chart scaffolding (full pattern)

```javascript
// 1. Dimensions
const margin = { top: 20, right: 30, bottom: 40, left: 50 };
const width = container.clientWidth;
const height = 400;
const innerWidth  = width  - margin.left - margin.right;
const innerHeight = height - margin.top  - margin.bottom;

// 2. SVG root (append once)
const svg = d3.select(container).append("svg")
  .attr("viewBox", `0 0 ${width} ${height}`)
  .style("width", "100%")
  .style("height", "auto");

const g = svg.append("g")
  .attr("transform", `translate(${margin.left},${margin.top})`);

// 3. Scales
const x = d3.scaleTime().domain(d3.extent(data, d => d.date)).range([0, innerWidth]);
const y = d3.scaleLinear().domain([0, d3.max(data, d => d.value)]).nice().range([innerHeight, 0]);

// 4. Axes
g.append("g").attr("class", "x-axis").attr("transform", `translate(0,${innerHeight})`).call(d3.axisBottom(x));
g.append("g").attr("class", "y-axis").call(d3.axisLeft(y));

// 5. Marks (data join)
const t = d3.transition().duration(500).ease(d3.easeCubicOut);

g.selectAll(".dot")
  .data(data, d => d.id)
  .join(
    enter => enter.append("circle").attr("class", "dot").attr("r", 0)
      .call(el => el.transition(t).attr("r", 4)),
    update => update.call(el => el.transition(t)
      .attr("cx", d => x(d.date))
      .attr("cy", d => y(d.value))),
    exit => exit.call(el => el.transition(t).attr("r", 0).remove())
  )
  .attr("cx", d => x(d.date))
  .attr("cy", d => y(d.value))
  .attr("fill", "steelblue");
```
