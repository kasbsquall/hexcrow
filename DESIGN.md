---
name: Hexcrow
description: A confidential milestone escrow shown as a four-part carbon invoice, one paper colour per viewer.
colors:
  original: "oklch(97% .008 95)"
  canario: "oklch(93% .07 98)"
  rosa: "oklch(91% .045 10)"
  azul: "oklch(91% .035 235)"
  carbon: "oklch(30% .03 255)"
  sello: "oklch(52% .16 25)"
  sello-ink: "oklch(45% .15 25)"
  verde: "oklch(46% .09 155)"
  desk: "oklch(84% .02 80)"
typography:
  display:
    fontFamily: "Figtree, sans-serif"
    fontSize: "76px"
    fontWeight: 650
    lineHeight: 0.9
    letterSpacing: "-0.035em"
    fontFeature: "\"tnum\", \"lnum\", \"zero\""
  headline:
    fontFamily: "Figtree, sans-serif"
    fontSize: "40px"
    fontWeight: 650
    lineHeight: 1.05
    letterSpacing: "-0.025em"
  title:
    fontFamily: "Figtree, sans-serif"
    fontSize: "22px"
    fontWeight: 650
    lineHeight: 1.15
    letterSpacing: "-0.02em"
  title-sm:
    fontFamily: "Figtree, sans-serif"
    fontSize: "18px"
    fontWeight: 650
    lineHeight: 1.3
    letterSpacing: "-0.015em"
  body:
    fontFamily: "Figtree, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
    fontFeature: "\"tnum\", \"lnum\", \"zero\""
  body-sm:
    fontFamily: "Figtree, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "Figtree, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.5
rounded:
  paper: "2px"
  dot: "50%"
  blot: "30px"
spacing:
  hair: "4px"
  xs: "6px"
  sm: "8px"
  md: "12px"
  box-x: "16px"
  lg: "24px"
  page-top: "32px"
  gutter: "40px"
  section: "56px"
  footer: "72px"
  page-bottom: "96px"
components:
  copy-tab:
    backgroundColor: "{colors.original}"
    textColor: "{colors.carbon}"
    typography: "{typography.label}"
    rounded: "{rounded.paper}"
    padding: "10px 14px 12px"
  form-box:
    textColor: "{colors.carbon}"
    typography: "{typography.body}"
    padding: "12px 16px 14px"
  action-chip:
    textColor: "{colors.carbon}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.paper}"
    padding: "7px 12px"
  invoice-row:
    backgroundColor: "{colors.original}"
    textColor: "{colors.carbon}"
    rounded: "{rounded.paper}"
    padding: "10px 14px"
  stub:
    backgroundColor: "{colors.original}"
    textColor: "{colors.carbon}"
    padding: "12px 14px 12px 50px"
  shell-bar:
    backgroundColor: "{colors.carbon}"
    textColor: "{colors.original}"
    height: "52px"
  not-on-copy:
    textColor: "{colors.carbon}"
    typography: "{typography.label}"
    rounded: "{rounded.paper}"
    padding: "3px 8px 3px 6px"
---

# Design System: Hexcrow

## Overview

**Creative North Star: "Carbon triplicate"**

Hexcrow is drawn as the multi-part NCR invoice book every freelancer and accounts office has filled: a white original with canary, pink and blue copies underneath, carbon-blue ink, boxes ruled into a grid, and a rubber stamp for the verdict. Selective disclosure maps one to one onto the object. Each viewer holds a different copy, and the boxes that did not transfer to that copy are the data its holder may not see.

The surface is a dense, light, documentary object on a desk-coloured ground with a faint paper grain. One form about 900px wide carries the invoice; a 360px aside holds the invoices in the book and a tear-off list of transaction stubs. Everything is set in one humanist sans, in carbon ink, with tabular figures. Depth comes from paper physically stacking (the other copies peek out under the active sheet), from perforations, and from a small set of soft, carbon-tinted shadows. Motion is paper motion: sheets slide, stamps land.

The direction rejects the dashboard of cards. The form is one ruled object, never a grid of floating tiles.

**Key Characteristics:**
- Paper colour encodes the viewer; the whole ground tints with it.
- Carbon-blue ink for all text; red ink only for verdicts and failures.
- Ruled boxes with hairline rules, 2px corners, no card chrome.
- Hidden values are smudges of fixed size, never shapes of the data.
- Every public fact is read live from the indexer, with skeletons while it loads and an explicit recorded fallback.

## Colors

Four paper stocks, one ink, one stamp red, one confirmation green, on a warm desk.

### Primary
- **Carbon Ink** (carbon): every piece of text, ruled lines (as alpha mixes), the shell bar background, the "Your path" tag and the stamp when the verdict is a neutral step (FUNDED, ISSUED, APPROVED, at 0.8 opacity).

### Secondary
- **Stamp Red** (sello): the SETTLED, REFUNDED and REJECTED stamps, the offline pulse in the bar, the dot beside a recorded (not live) status, and the text-selection tint.
- **Stamp Red, Ink** (sello-ink): the darker red used for text: rejection reasons on stubs, a failed hash match, a "bad" verdict, and the transparent-chain column in the comparison table.

### Tertiary
- **Ledger Green** (verde): confirmation only. The "read live from the book" dot and verdicts confirmed on the indexer.

### Neutral
- **Original** (original): the public copy; also the base paper for stubs, the selected invoice row and the light text on the carbon bar.
- **Canary** (canario): the client's copy; also the live pulse dot in the bar and the bar's focus ring.
- **Pink** (rosa): the contractor's copy.
- **Blue** (azul): the accountant's copy.
- **Desk** (desk): the table the pad rests on. The page ground is `color-mix(in oklab, <current copy> 38%, desk)`.
- **Rules**: carbon at 20% (hairline) and 42% (strong) over transparent. Secondary ink is carbon mixed 72 to 80% into the current paper, so it stays legible on every copy.

### Named Rules
**The Paper Per Viewer Rule.** Each viewer owns one paper: Public is Original, Client is Canary, Contractor is Pink, Accountant is Blue. Switching viewer sets `data-view` on the root and re-points `--copy`; the sheet, the ground and the "Your path" tag all derive from it. A new screen for a viewer uses that viewer's paper and never another's.

**The Stamp Is A Verdict Rule.** Red stamps and red ink speak only for verdicts and failures: settled, refunded, rejected, a mismatch, an unreachable indexer. Neutral progress (issued, funded, approved, committed) stamps in carbon. Red is never decoration, never a button fill, never a heading.

## Typography

**Display Font:** Figtree (with sans-serif)
**Body Font:** Figtree (with sans-serif)

**Character:** One quiet humanist sans does everything, as on a printed form; hierarchy comes from extreme scale contrast, not from a second family.

### Hierarchy
- **Display** (650, 76px, 0.9, -0.035em; 64px under 720px): the amount on a copy that may see it. Paired with a 12px label in the same box, a 6x jump.
- **Headline** (650, 40px, 1.05, -0.025em; 30px under 720px): the page h1 only, balanced wrap.
- **Title** (650, 22px, -0.02em): the form's "Hexcrow invoice" title. The invoice number sits at 600, 28px.
- **Title small** (650, 18px, -0.015em): aside and section headings, status and deadline values. Box headings inside the form step down to 15px.
- **Body** (400 to 500, 14px, 1.5): box values; invoice text runs at 18px, 1.4, max 46ch. Lead paragraphs 15px, max 62ch, pretty wrap.
- **Label** (600, 12px): the printed caption at the top of every ruled box, always with a Phosphor Light icon. Meta lines 12px at 400; the live/recorded note 11px at 600.

### Named Rules
**The Tabular Ink Rule.** The body sets `tabular-nums lining-nums slashed-zero` globally. Every number on a form, stub or bar stays tabular and carries its unit (tTARI, µT, epoch).

**The Identifier Rule.** Hashes, keys and ids are shortened with an ellipsis in the middle, keep the full value in `title`, and wrap anywhere. Epochs print without thousands separators; amounts and fees use them.

## Layout

A centred page, max 1320px, padding 32px 40px 96px. Above 1180px, two columns: the form at `minmax(0, 900px)` and a 360px aside, 40px apart; the comparison table sits under the form. At 1180px and below it collapses to one column (form, aside, comparison) and the registration marks disappear. At 720px and below the page padding drops to 22px 16px 64px.

Inside the form, a six-column grid of ruled boxes (spans 1, 2, 3, 4, 6). Under 720px it becomes two columns: number and status share a row, everything else spans both.

Spacing groups tight and separates wide: 4px between tabs and invoice rows, 6 to 12px inside boxes, 24px under the intro, 40px gutter, 56px above the comparison, 72px above the footer, 96px at the page foot. The 4px to 96px range keeps a 1:24 ratio.

## Elevation & Depth

Depth is physical paper. The active sheet lies on top of the other three copies, offset 7, 14 and 21px down and right in their own paper colours (only the first remains under 720px). Tabs and stubs carry perforations made of ground-coloured dots. A fixed SVG noise layer at 0.5 opacity, multiply, grains the whole page. Shadows are few, soft and always tinted with carbon, never black.

### Shadow Vocabulary
- **Sheet** (`0 1px 0 carbon 10%, 0 14px 30px -18px carbon 45%`): the active form only.
- **Stub and selected row** (`0 8px 18px -16px carbon 60%`): transaction stubs and the current invoice row (the row adds a 1px strong-rule ring).
- **Under-copy edge** (`0 1px 0 carbon 12%`): the bottom edge of each copy peeking out below.

### Named Rules
**The Stacked Pad Rule.** Depth means another sheet of paper. Offsets between copies are the world's own device and stay in the paper colours; never replace them with a generic drop shadow.

## Shapes

Every surface takes the same 2px corner: tabs (top corners only), the sheet (bottom corners only), chips, rows, skeletons, the focus ring. Circles are reserved for status dots and the stub index ring. The only soft shape is the 30px-radius blurred blot that stands in for a hidden figure. Boxes have no borders of their own; they are ruled by 1px inset hairlines shared with their neighbours, under a strong top rule.

## Components

### Copy tabs
Torn stubs of a pad. A four-column row, each tab in its own viewer's paper, icon plus name plus a one-line role ("Checks, cannot spend"). Unselected tabs sit 6px lower with a perforated bottom edge; hover lifts to 2px, press to 4px with scale 0.99, selected sits flush and joins the sheet. Arrow keys, Home and End move between them. Under 720px the role line hides and the icon stacks over the name.

### Ruled box (form field)
Padding 12px 16px 14px (10px 14px 12px on mobile). A 12px semibold label with an icon, then the value. Status boxes add a line saying where the fact came from.

### Hidden box (not on this copy)
A box whose data did not transfer keeps its label, fills with an SVG carbon smudge stretched to the box, and shows a "Not on this copy" chip with a lock icon plus screen-reader text naming what is hidden. A hidden amount draws a fixed 150 by 58px blurred blot next to the unit.

### Stamp
Inline SVG, double ruled frame and spaced capitals in Figtree 700, roughened with an ink displacement filter, rotated -8 degrees over the form's top edge (200px wide; 136px at the top right on mobile). Mini stamps (84px, COMMITTED or REJECTED) sit on each stub, rotated -6 or 5 degrees.

### Action chips
Inline chips with icon, action, state and, once done, a link to its transaction. Available: 1px strong rule on a lightened paper. Not available: dashed rule, transparent, secondary ink.

### Invoice rows (navigation)
Full-width buttons on 72% original paper; hover goes to full original, press scales to 0.98, the current one gets the ring and stub shadow and `aria-current`. A funded invoice's hourglass turns slowly.

### Transaction stubs
A tear-off list: numbered ring, what happened, who signed an attack, the rejection reason in red ink, a tx link and fee, and the live verdict with its icon. Perforations between stubs.

### Shell bar
Carbon bar, 52px (48px mobile), cells divided by 16% original hairlines: brand, guided-mode note, network state with pulse, mute toggle.

### Skeleton
A 2px-radius bar of carbon at 12%, pulsing to 0.45 opacity every 1.6s, with `role="status"` and a label saying what is being read.

### Motion and sound
Easings: out `cubic-bezier(.23,1,.32,1)`, move `cubic-bezier(.77,0,.175,1)`. Switching copy or invoice: the old sheet slides 56px down, tilts 0.8 degrees and fades (240ms, move), the new one rises from 18px (240ms, out), and the stamp lands from 1.04 through 0.95 to 1 (220ms, 140ms delay). Entrance: blocks rise 6px over 260ms with 60ms steps; stubs over 240ms with 30ms per row, capped at 8, played once per invoice. Tabs 160ms, rows and mute 150ms, ground colour 240ms, pulse beat 280ms on each successful indexer read. Ambient loops: registration marks drift over 17 and 22s, the funded hourglass turns every 6s. `prefers-reduced-motion` removes every animation and transition and skips the opening tour.

Three Kenney CC0 clips at volume 0.28, played only from a user gesture and silenced by a mute toggle remembered in local storage: `switch.ogg` when the visitor changes copy (not during the automatic tour), `stamp.ogg` when the visitor opens another invoice, `verified.ogg` when a visitor-triggered recheck confirms every verdict.

## Do's and Don'ts

### Do:
- **Do** give every viewer its own paper (Public original, Client canary, Contractor pink, Accountant blue) and derive the sheet and ground from `--copy`.
- **Do** draw hidden values as value-independent smudges: a smudge stretched to the box and a fixed 150 by 58px blot, plus the "Not on this copy" chip and screen-reader text.
- **Do** read status, epoch and verdicts live from the indexer, show a labelled skeleton while reading, and on failure fall back to the recorded run with an explicit "recorded" or "indexer unreachable" note and the red dot.
- **Do** put a Phosphor Light icon on every box label, stub verdict, action and table row.
- **Do** keep every motion under 300ms on transform and opacity, with the reduced-motion override.

### Don't:
- **Don't** stamp anything that is not a verdict, and don't use stamp red for decoration, fills or headings.
- **Don't** size, shape or blur a hidden value from its real length or magnitude; the smudge must look the same for 1 tTARI and 1,000,000.
- **Don't** paint a default value (0, "Issued", an empty hash) while the indexer is answering.
- **Don't** split the form into floating cards with their own shadows and radii; it stays one ruled sheet.
- **Don't** play a sound without a user gesture or with the toggle off.
