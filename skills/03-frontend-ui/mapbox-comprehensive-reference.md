---
name: Mapbox Comprehensive Reference
description: High-density reference for Mapbox development across Web (GL JS v3), Android, and iOS. Covers installation, framework integration (React, Vue, Svelte, Angular, Next.js), cartography, data visualization, search, geospatial operations, performance optimization, and migration from Google Maps/MapLibre. Use this as the primary technical cue for any Mapbox-related task.
---

# Mapbox Comprehensive Reference

## 1. Core Philosophy & Migration

### Declarative vs. Imperative
- **Google Maps (Imperative):** Create objects (Marker, Polygon), update via setters (`marker.setPosition()`), add to map with `.setMap(map)`.
- **Mapbox (Declarative):** Define **Data Sources** (GeoJSON, Vector Tiles) and **Layers** (visual representation). Update data (`source.setData()`), not individual objects. Mapbox treats everything as data + styling.

### Migration Quick Reference
- **Coordinate Order:** Mapbox uses `[longitude, latitude]` (GeoJSON standard). **CRITICAL:** Reversing this is the #1 migration bug.
- **Performance:** Mapbox uses WebGL. It is 10-100x faster for large datasets (10,000+ points) compared to DOM-based Google Maps markers.
- **Checklist:** ✅ Swap coordinates (lat,lng → lng,lat) ✅ Use Symbol layers for 100+ markers ✅ Wait for `load` event before adding layers ✅ Implement proper cleanup (`map.remove()`).

| Aspect | Google Maps | MapLibre GL JS | Mapbox GL JS |
| :--- | :--- | :--- | :--- |
| **Philosophy** | Imperative | Declarative | Declarative |
| **Rendering** | DOM/Canvas | WebGL | WebGL 2 (v3+) |
| **License** | Proprietary | BSD 3-Clause | Proprietary (v2+) |
| **Tiles** | Google Tiles | OSM / Custom | Premium Mapbox Tiles |
| **APIs** | Separate | Third-party | Integrated Ecosystem |

---

## 2. Platform Setup & Installation

### Web (Mapbox GL JS v3)
```bash
npm install mapbox-gl@^3.0.0
# Search JS (optional)
npm install @mapbox/search-js-react # or @mapbox/search-js-web
```
- **CSS Import:** `import 'mapbox-gl/dist/mapbox-gl.css';` (Critical for markers/popups).
- **Prototyping (CDN):** `<script src="https://api.mapbox.com/mapbox-gl-js/v3.x.x/mapbox-gl.js"></script>`

### Android (Maps SDK v11)
- **Maven Repo:** `https://api.mapbox.com/downloads/v2/releases/maven`
- **Dependency:** `com.mapbox.maps:android:11.x.x`
- **Token:** Store in `app/res/values/mapbox_access_token.xml`.

### iOS (Maps SDK v11)
- **SPM:** `https://github.com/mapbox/mapbox-maps-ios.git`
- **Token:** Add `MBXAccessToken` key to `Info.plist`.
- **Permissions:** Add `NSLocationWhenInUseUsageDescription` to `Info.plist`.

---

## 3. Framework Integration Patterns

### Core Lifecycle Rules
1. **Initialize Once:** Use mount hooks (`useEffect`, `onMounted`, `onMount`).
2. **Proper Cleanup:** **ALWAYS** call `map.remove()` on unmount to prevent memory leaks.
3. **Wait for Load:** All `addSource`/`addLayer` calls must be inside `map.on('load', ...)`.

### React (useRef + useEffect)
```jsx
const mapRef = useRef(null);
const mapContainerRef = useRef(null);

useEffect(() => {
  if (mapRef.current) return;
  mapRef.current = new mapboxgl.Map({
    container: mapContainerRef.current,
    accessToken: import.meta.env.VITE_MAPBOX_TOKEN,
    style: 'mapbox://styles/mapbox/standard',
    center: [-122.4, 37.8], zoom: 12
  });
  return () => mapRef.current?.remove();
}, []);
```

### Next.js (App Router)
- **Directive:** Use `'use client';` at the top of the component.
- **SSR Handling:** Mapbox requires `window`. Dynamic imports with `ssr: false` are recommended for the component.

### Vue 3 (Composition API)
```javascript
const mapContainer = ref(null);
let map = null;
onMounted(() => {
  map = new mapboxgl.Map({ container: mapContainer.value, ... });
});
onUnmounted(() => map?.remove());
```

---

## 4. Cartography & Style Design

### Visual Hierarchy
- **Primary:** User task data (Markers, Routes, POIs). Use bold colors, high contrast, larger symbols.
- **Secondary:** Navigation context (Major roads, labels).
- **Tertiary:** Background (Water, parks, minor roads).
- **Common Mistake:** Putting app data *below* base map POIs. App data must be the top-most layer.

### Style Patterns
- **Restaurant Finder:** Neutral background, high-contrast POI icons (#FF6B35 orange), visible street names.
- **Real Estate:** Property boundaries (#7e57c2 purple), price-coded fills (green→red), highlight parks/transit.
- **Navigation:** Thick blue route line (#2196f3), pulsing user location, turn arrows at maneuvers.
- **Data Viz Base:** Grayscale palette, minimal detail, major cities only.

### Expressions (Data-Driven Styling)
- **Interpolate:** Smooth transitions `['interpolate', ['linear'], ['get', 'value'], 0, '#fff', 100, '#f00']`.
- **Step:** Discrete buckets `['step', ['get', 'count'], '#blue', 10, '#yellow', 50, '#red']`.
- **Match:** Categorical `['match', ['get', 'type'], 'park', '#green', 'water', '#blue', '#gray']`.

---

## 5. Data Visualization & Layers

### Layer Types
- **Symbol:** Icons and text. GPU-accelerated. Use for >100 points.
- **Circle:** Simple points. Most performant for high-density.
- **Heatmap:** Density visualization. Use `heatmap-weight` and `heatmap-intensity`.
- **Fill-Extrusion:** 3D buildings/polygons. Use `fill-extrusion-height`.
- **Line:** Routes/boundaries. Use `line-dasharray` for patterns.

### Optimization Strategy
- **GeoJSON (< 5MB):** Load directly via `map.addSource('id', { type: 'geojson', data })`.
- **Vector Tiles (> 20MB):** Use for global/large datasets. Better performance, progressive loading.
- **Feature State:** Update styling without re-parsing geometry. Requires `generateId: true` on source.
```javascript
map.setFeatureState({ source: 'states', id: featureId }, { hover: true });
// Paint property: ['case', ['boolean', ['feature-state', 'hover'], false], '#red', '#blue']
```

---

## 6. Search & Geocoding

### Product Selection
- **Search Box API:** Use when POI data is needed. Session-based pricing.
- **Geocoding API:** Use for address geocoding only, no POIs, or permanent geocoding.

### Best Practices
- **Debouncing:** Wait 300ms before calling API (handled automatically by Search SDKs).
- **Proximity:** **ALWAYS** set `proximity: [lng, lat]` to bias results to user location.
- **Spatial Filters:** Use `bbox` for hard constraints (service areas) and `country` for regional limits.
- **Session Tokens:** (Direct API only) Use 1 token for `suggest` + `retrieve` to group costs.

---

## 7. Geospatial Operations (MCP & Turf.js)

### Decision Framework
- **Offline (Turf.js):** Straight-line distance, point-in-polygon, buffers, centroids, area. Instant, free.
- **Routing APIs:** Driving/Walking/Cycling distance, travel times, traffic-aware routes, route optimization (TSP). Costs API calls.

### Common Hybrid Patterns
1. **Isochrone + Containment:** Create 15-min drive-time polygon (API) → Check 500 addresses (Turf.js).
2. **Routing + Filter:** Get route geometry (API) → Create 500m buffer (Turf.js) → Filter POIs in buffer.

---

## 8. Performance Optimization

### Eliminate Waterfalls
- **Parallel Loading:** Fetch data *at the same time* as map initialization. Don't wait for `on('load')` to start fetching.
- **Code Splitting:** Use dynamic imports `const mapboxgl = await import('mapbox-gl')` to reduce initial bundle.

### Interaction & Rendering
- **Throttle Events:** Throttle `move` and `zoom` events to 16ms (60fps) or 100ms for heavy tasks.
- **Cluster Markers:** Use `cluster: true` for >10,000 points.
- **Simplify Expressions:** For 100k+ features, use simple property lookups; pre-calculate logic on server.
- **Layer Visibility:** Set `minzoom` and `maxzoom` to avoid rendering layers that aren't useful at specific scales.

---

## 9. Security & Token Management

### Rules
- **Public Tokens (`pk.*`):** Client-side. **MUST** have URL restrictions in Mapbox Dashboard.
- **Secret Tokens (`sk.*`):** Server-side only. NEVER expose in frontend code.
- **Environment Variables:** Always use `process.env` or `import.meta.env`. NEVER hardcode.
- **Principle of Least Privilege:** Create scoped tokens (e.g., search-only, read-only).

---

## 10. Troubleshooting Checklist
- [ ] **Coordinates reversed?** Should be `[lng, lat]`.
- [ ] **CSS missing?** Import `mapbox-gl.css`.
- [ ] **Map not visible?** Check container height/width (must be explicit).
- [ ] **Memory leak?** Ensure `map.remove()` is called on cleanup.
- [ ] **Style not loading?** Check token scopes and URL restrictions.
- [ ] **Operation failed?** Ensure it is inside `map.on('load', ...)`.
- [ ] **Performance jank?** Switch from HTML markers to Symbol layers.

## Resources
- [Official Docs](https://docs.mapbox.com)
- [Style Specification](https://docs.mapbox.com/mapbox-gl-js/style-spec/)
- [Turf.js Docs](https://turfjs.org)
