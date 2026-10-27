# UI source provenance

- React, React DOM, Vite and the Tauri API/CLI are resolved from the npm `latest` stable tags and pinned by the lockfile.
- shadcn/ui: current components generated with CLI `4.22.1` from https://ui.shadcn.com. MIT. Shared components live in `src/packages/ui/src/ui`.
- Shaders: official `shaders` package `4.0.4`; the HUD rim uses its `shaders/core` renderer and `shaders/std` `defineShader`; https://shaders.com. The installed package's MIT license applies to the code dependency. No paid presets or account exports are used.
- MCP: official `@modelcontextprotocol/server` / `client` `2.3.1`, current stable v2 line for the `2026-07-28` specification. https://ts.sdk.modelcontextprotocol.io/v2/.

Registry-generated utility imports are adapted to `@agent-hud/ui/lib/utils` so the UI package does not import app-private modules. Shader animation is disabled with reduced motion or when the document is hidden.

- Original Agent HUD UI layouts, geometry, hover behavior and bot logic are ports from `agent-hud-native-open`, licensed under Apache-2.0. The license is included at the repository root in [LICENSE](../LICENSE). Native rendering expressions were translated to React, Canvas, SVG and Rust.
- React Bits AnimatedContent: official TypeScript source from https://github.com/DavidHDev/react-bits/tree/main/src/ts-default/Animations/AnimatedContent, retrieved 2026-10-19, retained unmodified with its upstream MIT + Commons Clause license in `src/packages/ui/src/react-bits/LICENSE.md`. It uses gsap 3.15.0.
- Original provider PNGs and the Clawd sprites are retained under src/packages/hud/src/assets with original notices; the Settings wallpaper is in src/packages/ui/src/assets and the application icon in src/packages/app/src/assets. Clawd sprites retain their separate Anthropic copyright; see src/packages/hud/src/assets/README.md and THIRD_PARTY_NOTICES.txt.

shadcn visual overrides live in src/packages/ui/src/global.css; registry component behavior is unchanged. HUD geometry and timelines are product implementation code. Custom Shader rim effects implement the original six glow styles and six effects; they retain a static fallback when WebGPU is unavailable.

`src/packages/vite-config/src/shader-lifecycle.ts` applies version- and source-anchor-checked
lifecycle fixes to the official shaders 4.0.4 React adapter at Vite build time.
It disposes initialization that completes after unmount and prevents duplicate
pending initialization during StrictMode effect replay. Installed upstream files remain unchanged. The HUD rim no longer uses the React adapter: `HUDGlow` owns the renderer, keeps its display-rate frame loop off and asks for frames itself, so it needs no frame-cap patch. Review/remove this temporary patch when upgrading.
