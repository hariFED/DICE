# DICE Node Enclosure — 3D Print Ideas

## Key Constraints
- ESP32-S3-N16R8 DevKit (~68mm x 25mm x 10mm)
- USB-C port must remain accessible
- Onboard WiFi antenna — **no metal above the antenna area** (top-right of board)
- WS2812 RGB LED on GPIO 48 must be visible
- GPIO 1 (ADC pin) must stay floating — no metal contact
- Ventilation for ~120mA heat dissipation
- Must be cheap to print (minimal material, no supports ideally)

---

## Idea 1: "Puck" — Minimal Round Shell (RECOMMENDED — cheapest)

A flat, round disc enclosure. Think hockey puck meets smart-home device.

- **Shape:** Cylinder, ~80mm diameter, ~20mm tall
- **Two halves:** Top lid + bottom base, snap-fit clips (no screws)
- **LED:** Small light-pipe channel or a 3mm hole in the top lid aligned with the WS2812 — LED glows through
- **USB:** Slot cut in the side wall
- **Ventilation:** Ring of small slots around the bottom edge (hidden underneath)
- **Antenna:** Top lid has a thin section (0.8mm) or cutout over the antenna zone
- **Print:** Both halves print flat, no supports needed. ~15g of filament total
- **Cost:** ~$0.30-0.50 in PLA per unit
- **Finish:** Matte black PLA, looks clean as-is. Optional: light sand + spray for premium feel

**Why cheapest:** Minimal plastic, zero supports, prints in ~45 min, snap-fit = no hardware.

---

## Idea 2: "Monolith" — Rectangular Slab

A slim rectangular box, like a tiny streaming stick or a thick credit card.

- **Shape:** ~85mm x 40mm x 18mm rounded rectangle
- **Two halves:** Top + bottom, 2x M2 screws or snap-fit
- **LED:** Translucent strip or diffuser window on the front face — can use a printed-in-place thin wall (0.4mm) as a light diffuser
- **USB:** Slot on the short side
- **Ventilation:** Hexagonal pattern on the bottom
- **Branding:** "DICE" debossed (recessed text) on the top — free, no post-processing
- **Print:** Flat on bed, no supports. ~18g filament
- **Cost:** ~$0.40-0.60 in PLA

---

## Idea 3: "Crystal" — Geometric Low-Poly

A faceted, gem-like shape that looks high-end but is actually simple geometry.

- **Shape:** Truncated icosahedron-ish (not a full sphere — more like a D20 with a flat base)
- **Split:** Horizontal split at the equator, snap-fit
- **LED:** Light bleeds through seams between facets — no separate window needed, just thin walls at the LED position
- **USB:** Port exit at the back flat face
- **Ventilation:** Gaps between faceted panels
- **Print:** Needs some supports on the top half (~20% more filament). ~22g total
- **Cost:** ~$0.50-0.80 in PLA
- **Premium feel:** Looks $50, costs $0.50. Great for marketing photos.

---

## Idea 4: "Stack" — Layered Sandwich

Inspired by Raspberry Pi cases — multiple thin layers stacked together.

- **Construction:** 3-4 laser-cut-style layers (but 3D printed), held by 4x M3 bolts through the corners
- **Layers:** Base plate → board spacer → board holder → top plate
- **LED:** Visible through gap between layers
- **USB:** Accessible through gap between layers (no special cutout)
- **Ventilation:** Excellent — open air between every layer
- **Print:** Each layer prints flat in minutes. Trivially parametric — easy to adjust for board revisions
- **Cost:** ~$0.25-0.40 in PLA + $0.20 in M3 hardware
- **Pro:** Most hackable, easiest to iterate

---

## Idea 5: "Wall-Mount Wedge" — Plug-Friendly

Designed to stick/mount near a USB power outlet. Wedge shape, angled face.

- **Shape:** Triangular prism, ~70mm x 40mm x 30mm, with a flat back and angled front face
- **Mount:** Flat back with keyhole slot for wall screw, or 3M VHB tape pad
- **LED:** Angled face means the LED is always visible from room level
- **USB:** Cable exits from the bottom (gravity-friendly routing)
- **Ventilation:** Slots along the bottom edge
- **Print:** Prints on the flat back, no supports. ~16g
- **Cost:** ~$0.35-0.55 in PLA
- **Pro:** Designed for "set and forget" deployment. Node stays out of the way.

---

## Cost Reduction Tips (apply to any design)

| Technique | Savings |
|-----------|---------|
| Use PLA instead of PETG/ABS | 30-40% cheaper filament |
| 1.2mm walls (2 perimeters at 0.6mm nozzle) | Uses ~40% less plastic than standard 2mm walls |
| Snap-fit clips instead of screws | Saves $0.05-0.20/unit in hardware |
| Debossed text instead of printed labels | Free branding, no stickers |
| Print bottom-up, flat base, no supports | Saves 15-20% filament + time |
| Batch print (12-16 per plate on 300mm bed) | Reduces per-unit time significantly |
| Thin-wall LED diffuser (0.4mm) in white PLA | No separate diffuser part needed |

## Estimated Production Cost per Unit

| Volume | Enclosure Only | + Assembly |
|--------|---------------|------------|
| 1-10 units | $0.50-1.00 | $0.50-1.00 |
| 50-100 units | $0.30-0.50 | $0.30-0.50 |
| 500+ units | $0.20-0.35 | Consider injection molding at this scale |

## My Recommendation

**Go with the Puck (Idea 1) for V1.** It's the cheapest, fastest to print, needs zero hardware, and looks clean. Deboss "DICE" on the top + a small LED hole, done. Ship it.

If you want more visual wow for marketing, print a few Crystals (Idea 3) as hero units for photos/demos.
