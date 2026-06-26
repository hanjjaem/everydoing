# Apple Design System Summary

> 출처: VoltAgent/awesome-design-md (design-md/apple/DESIGN.md). 이 앱 UI 리프레시의 *참고 기준*.

This comprehensive design documentation outlines Apple's web design language across five surfaces: homepage, environment, store, iPhone 17 Pro configurator, and accessories pages.

## Core Principles
Apple's approach prioritizes **photography over UI chrome**. Product imagery is the focal point; interface elements deliberately recede. "A photography-first interface that turns marketing into a museum gallery" — minimal decoration, no gradients, restraint in visual hierarchy.

## Color Strategy
Single accent color (**Action Blue, #0066cc**) for all interactive elements. Surfaces alternate light (white, parchment off-white) and near-black tiles (#272729–#252527). Text: near-black ink (#1d1d1f) on light, white on dark. No secondary brand colors; every clickable element shares the same blue.

## Typography
Body 17px (SF Pro Text, weight 400) at 1.47 line-height — larger than typical SaaS. Headlines: SF Pro Display with **negative letter-spacing** (-0.28px to -0.374px) for the "Apple tight" compression. Weight 600 dominates headlines; weight 500 is intentionally absent (ladder: 300 / 400 / 600 / 700).

## Layout & Spacing
Full-bleed product tiles stack edge-to-edge at 80px vertical padding; color changes (not borders) divide sections. Grid: 3–5 column utility cards on store/accessories. Generous whitespace: ≥40px around product renders, ~64px above headlines.

## Elevation
**Exactly one drop-shadow** (rgba(0,0,0,0.22) 3px 5px 30px), only on product imagery resting on surfaces. Elsewhere elevation comes from surface-color shifts and backdrop-filter blur on sticky nav — never shadows on cards/buttons.

## Key Components
- Buttons: pill primary CTAs, compact utility rectangles (rounded.sm), ghost variants
- Product tiles: light/parchment/dark alternations, centered headline, tagline, dual CTAs, render
- Navigation: 44px global nav (black) + frosted sub-nav with backdrop blur
- Cards: 18px radius utility cards with 1px hairline borders
- Footer: dense link columns at 2.41 line-height

## Responsive Strategy
Content locks at 1440px desktop max. Breakpoints: 1068px (small desktop), 834px (tablet landscape), 640px (phone), 419px (small phone). Hero type scales 56px → 28px; grids compress 5-col → single-col on mobile.
