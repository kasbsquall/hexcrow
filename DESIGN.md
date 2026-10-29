---
name: Hexcrow
description: A confidential milestone escrow shown as a five-part carbon invoice on a desk at night, one paper colour per viewer, kept by a crow.
colors:
  original: "oklch(95% .012 85)"
  canario: "oklch(91% .07 96)"
  rosa: "oklch(89% .045 15)"
  salvia: "oklch(89% .04 160)"
  azul: "oklch(89% .035 240)"
  carbon: "oklch(26% .03 262)"
  sello: "oklch(54% .17 25)"
  sello-ink: "oklch(45% .15 25)"
  verde: "oklch(44% .09 155)"
  night: "oklch(17.5% .025 265)"
  night-2: "oklch(21.5% .03 265)"
  night-3: "oklch(27% .032 265)"
  bone: "oklch(92% .016 85)"
  bone-2: "oklch(76% .022 262)"
  bone-3: "oklch(64% .025 262)"
  night-rule: "oklch(92% .016 85 / .12)"
  night-rule-strong: "oklch(92% .016 85 / .24)"
  candle: "oklch(83% .12 80)"
  sello-night: "oklch(68% .16 27)"
  verde-night: "oklch(76% .1 155)"
typography:
  display:
    fontFamily: "Figtree, sans-serif"
    fontSize: "76px"
    fontWeight: 650
    lineHeight: 0.9
    letterSpacing: "-0.035em"
    fontFeature: "\"tnum\", \"lnum\", \"zero\""
  tally:
    fontFamily: "Figtree, sans-serif"
    fontSize: "56px"
    fontWeight: 650
    lineHeight: 0.9
    letterSpacing: "-0.04em"
    fontFeature: "\"tnum\", \"lnum\", \"zero\""
  headline:
    fontFamily: "Figtree, sans-serif"
    fontSize: "44px"
    fontWeight: 650
    lineHeight: 1.02
    letterSpacing: "-0.028em"
  number:
    fontFamily: "Figtree, sans-serif"
    fontSize: "28px"
    fontWeight: 600
    lineHeight: 1.05
    letterSpacing: "-0.02em"
    fontFeature: "\"tnum\", \"lnum\", \"zero\""
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
  lead:
    fontFamily: "Figtree, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.5
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
    lineHeight: 1.5
  label:
    fontFamily: "Figtree, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.5
  micro:
    fontFamily: "Figtree, sans-serif"
    fontSize: "11px"
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
  page-top: "36px"
  gutter: "40px"
  column-gap: "48px"
  section: "64px"
  section-lg: "72px"
  footer: "80px"
  page-bottom: "96px"
components:
  shell-bar:
    backgroundColor: "{colors.night}"
    textColor: "{colors.bone}"
    typography: "{typography.body-sm}"
    height: "52px"
  copy-tab:
    backgroundColor: "{colors.original}"
    textColor: "{colors.carbon}"
    rounded: "{rounded.paper}"
    padding: "10px 12px 12px"
  sheet:
    backgroundColor: "{colors.original}"
    textColor: "{colors.carbon}"
    rounded: "{rounded.paper}"
  form-box:
    textColor: "{colors.carbon}"
    typography: "{typography.body}"
    padding: "12px 16px 14px"
  not-on-copy:
    textColor: "{colors.carbon}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.paper}"
    padding: "3px 8px 3px 6px"
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
    typography: "{typography.body-sm}"
    padding: "12px 14px 12px 50px"
  try-button:
    backgroundColor: "{colors.night-2}"
    textColor: "{colors.bone}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.paper}"
    padding: "8px 14px"
  try-button-hover:
    backgroundColor: "{colors.night-3}"
  try-all:
    textColor: "{colors.bone}"
    typography: "{typography.body}"
    rounded: "{rounded.paper}"
    padding: "10px 16px"
  try-all-hover:
    backgroundColor: "{colors.night-2}"
  night-button:
    textColor: "{colors.bone}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.paper}"
    padding: "6px 12px"
  night-button-hover:
    backgroundColor: "{colors.night-2}"
---

# Design System: Hexcrow

## Overview

**Creative North Star: "Carbon copies at midnight"**

Hexcrow is drawn as the multi-part carbon invoice pad every freelancer and accounts office has filled, lying on a desk at night. The pad has five copies of one invoice, each on its own paper stock, written in carbon-blue ink and ruled into boxes. Selective disclosure maps one to one onto the object: each viewer holds a different copy, and what did not transfer to that copy is a carbon smudge with a lock. A rubber stamp gives the invoice its status, a wax seal sits on the escrow while it holds the money and shows a crack once it is spent, and a crow perches on the copy tabs.

The paper stays light so the form reads; the night is around it. The desk is a dark, cool ground (never pure black) lit by two soft radial glows and grained by a fixed noise layer, with bone-coloured text and hairline rules. One form carries the invoice, a 360px aside holds the invoices in the book and a tear-off list of transaction stubs, and below them the "Try to steal it" list replays every attack the chain refused. Everything is set in Figtree with tabular figures. Depth is paper physically stacking: the other copies peek out under the active sheet. Motion is paper motion: sheets slide, stamps land, the crow pecks.

The form is one ruled object. It is never split into floating tiles with their own shadows.

**Key Characteristics:**
- Paper colour encodes the viewer; the sheet re-points `--copy` when the viewer changes.
- Two inks: carbon on paper, bone on night. Each status colour has a paper version and a night version.
- Ruled boxes with shared hairlines, one 2px corner everywhere, no card chrome.
- Hidden values are smudges and blots of fixed size, never shapes of the data.
- Every public fact is read live from the indexer, with labelled skeletons while it loads and an explicit recorded fallback.
- Candle yellow is the only light on the desk.

## Colors

Five paper stocks, one carbon ink and a stamp red on the paper; a cool night desk, bone ink and one candle on the ground.

### Primary
- **Candle** (candle): the only warm light on the night. The focus ring on the desk, the live network pulse and its glow, the ring around the current invoice in the book, the icons beside "What the escrow adds", the crow's eye, the text selection tint (34% mix), and a THROUGH verdict if an attack were ever accepted.

### Secondary
- **Stamp Red** (sello): the SETTLED and REFUNDED stamps on the sheet, the REFUSED mini stamp on stubs, the wax of the escrow seal, and the dot beside a recorded (not live) status.
- **Stamp Red, Ink** (sello-ink): red for text on paper: rejection reasons on stubs, a failed hash match, a "bad" verdict.
- **Stamp Red, Night** (sello-night): red on the desk: the REFUSED stamp and "Refused by the chain" verdict in the attack list, the offline pulse, the transparent-chain column of the comparison table.

### Tertiary
- **Ledger Green** (verde): confirmation on paper only: the "Read live from the book" dot, confirmed verdicts on stubs, the contractor's pre-check ticks.
- **Ledger Green, Night** (verde-night): "None got through" under the attack tally.

### Neutral
- **Original** (original): the public copy; also the base paper for stubs and the current invoice row.
- **Canario** (canario): the client's copy.
- **Rosa** (rosa): the contractor's copy.
- **Salvia** (salvia): the mediator's copy.
- **Azul** (azul): the accountant's copy.
- **Carbon** (carbon): all text on paper, the rules on paper (as 20% and 42% mixes over transparent), the "yours" tag, the focus ring on paper, and the stamp for neutral steps (ISSUED, FUNDED, APPROVED at 0.8 opacity; COMMITTED at 0.78). Secondary ink on paper is carbon mixed 70 to 74% into the current paper.
- **Night** (night): the desk. **Night 2** (night-2) is the hover and button fill on the desk; **Night 3** (night-3) the stronger hover and the scrollbar thumb. Black enters only as a mix: the shell bar is night mixed 70% with black, and the seal rim is sello-ink mixed 80% with black.
- **Bone** (bone): primary text on the night. **Bone 2** (bone-2) for secondary text and leads; **Bone 3** (bone-3) for the quietest labels, table heads and the waiting pulse.
- **Night rules** (night-rule at 12%, night-rule-strong at 24%): bone hairlines that divide the bar cells, the intro proof, the attack rows, the table and the footer.

### Named Rules
**The Paper Per Viewer Rule.** Each viewer owns one paper: Public is Original, Client is Canario, Contractor is Rosa, Mediator is Salvia, Accountant is Azul. Switching viewer sets `data-view` on the root and re-points `--copy`; the sheet, its secondary ink and the "yours" tag derive from it. A new screen for a viewer uses that viewer's paper and never another's.

**The Two Inks Rule.** Carbon writes on paper, bone writes on night. A status colour placed on paper uses its paper form (sello, sello-ink, verde); placed on the desk it uses its night form (sello-night, verde-night, candle). Never put carbon text on the night or bone text on paper.

**The Stamp Is A Verdict Rule.** Red stamps and red ink speak only for verdicts and failures: settled, refunded, refused, a mismatch, an unreachable indexer. Neutral progress (issued, funded, approved, committed) stamps in carbon. Red is never decoration, never a button fill, never a heading.

**The One Candle Rule.** Candle yellow marks light and attention on the desk (focus, live pulse, current selection) and nothing else. It never fills a surface or sets body text.

## Typography

**Display Font:** Figtree (with sans-serif), weights 400 to 700 from Google Fonts
**Body Font:** Figtree (with sans-serif)

**Character:** One quiet humanist sans does everything, as on a printed form; hierarchy comes from extreme scale contrast and weight 650, never from a second family. The body sets `tabular-nums lining-nums slashed-zero` for the whole page.

### Hierarchy
- **Display** (650, 76px, 0.9, -0.035em; 64px under 720px): the amount on a copy that may see it, and the attack count in the intro proof (line-height 0.86, -0.04em). Sits next to a 12 or 13px label in the same block, a jump of about 6x.
- **Tally** (650, 56px, 0.9, -0.04em): the count of attacks tried in "Try to steal it".
- **Headline** (650, 44px, 1.02, -0.028em, max 15ch, balanced; 32px under 720px): the page h1 only.
- **Number** (600, 28px, 1.05, -0.02em): the invoice number on the form. Proof figures in the intro row run at 650, 24px.
- **Title** (650, 22px, -0.02em): the form title "Hexcrow invoice" and the section headings on the desk (-0.018em, with a 24px icon). The copy picker heading steps to 20px.
- **Title small** (650, 18px, -0.015em): aside heading; status and deadline values in the form run at 600, 18px. The caption that says what this copy is runs at 500, 17px, 1.42. Box headings inside the form (access, rules) and invoice row names step down to 650, 15px.
- **Lead** (400, 16px, 1.5, max 60ch, pretty wrap; 15px under 720px): the intro paragraph. Section leads on the desk run at 14px, max 66ch.
- **Body** (400 to 500, 14px, 1.5): box values. Invoice text runs at 500, 18px, 1.4, max 46ch. Attack titles run at 550, 15px.
- **Body small** (13px): stubs, chips, buttons, table cells, footer, bar text.
- **Label** (600, 12px): the printed caption at the top of a ruled box, with a Phosphor Light icon (15px) on every label except the invoice number. Also meta lines (400), definition terms and table heads.
- **Micro** (600, 11px): the live/recorded note, the "used" and "yours" tags, proof row terms. Copy tab names drop to 11px under 720px.

Stamps set their word in Figtree 700, 27 units, letter-spacing 3, uppercase, inside the stamp SVG. Uppercase appears nowhere else.

### Named Rules
**The Tabular Ink Rule.** Every number on a form, stub, bar or tally stays tabular and carries its unit (tTARI, µT, epoch).

**The Identifier Rule.** Hashes, keys and ids wrap anywhere with 0.01em tracking, and are shortened in the middle with the full value kept in a title. Fees and amounts use thousands separators.

## Layout

A centred page, max 1320px, padding 36px 40px 96px, over a full-bleed shell bar. The intro is two columns (text and a 380px proof block, 40px apart, bottom aligned). The work area is a grid of the copy viewer at `minmax(0, 1fr)` and a 360px aside, 48px apart; "Try to steal it", "What the escrow adds" and the transparent-chain comparison follow under the viewer while the aside runs down the right.

At 1180px and below everything collapses to one column (viewer, aside at max 640px, attacks, additions, comparison). At 720px and below the page padding drops to 24px 16px 64px, the bar hides its guided-mode note, the tabs become five auto-width columns with the icon over the name and no role line, the form grid becomes two columns (number and status share a row), only one under-copy remains, the stamp moves to the top right at 136px, and the comparison table becomes stacked blocks with "Transparent:" and "Hexcrow:" prefixes.

Inside the form, a six-column grid of ruled boxes spans 1, 2, 3, 4 or 6 columns. The amount and text boxes hold a minimum height of 150px (120px on mobile) so a hidden value takes the same room as a visible one.

Spacing groups tight and separates wide: 4px between tabs and invoice rows, 6 to 12px inside boxes, 18px under the copy picker heading, 40px under the intro, 48px between viewer and aside, 64px above the attacks, 72px above each later section, 80px above the footer, 96px at the page foot. The range from 4px to 96px is a 1:24 ratio.

## Elevation & Depth

Depth is physical paper on a dark desk. The active sheet lies on top of the other copies, offset 7, 14 and 21px down and right in their own paper colours at 80% brightness (Canario, Rosa, Azul; the active viewer's own stock is swapped for Original). Unselected tabs sit 6px lower at 86% brightness. Tabs and stubs carry perforations made of night-coloured dots. The desk itself has two radial glows (a cool blue pool behind the form and a faint warm one at the top right) and a fixed SVG noise layer at 0.55 opacity in soft-light. Shadows are few and tinted with the night hue, never pure black.

### Shadow Vocabulary
- **Sheet** (`box-shadow: 0 30px 60px -30px oklch(5% .02 265 / .9), 0 0 0 1px oklch(5% .02 265 / .3)`): the active form only; it lifts the paper off the desk.
- **Box rules** (`box-shadow: 1px 0 0 var(--rule), 0 1px 0 var(--rule)`): the shared hairlines between ruled boxes, drawn as shadows so neighbours share one line.
- **Current invoice ring** (`box-shadow: 0 0 0 2px var(--candle)`): the invoice being shown in the book.
- **Live pulse glow** (`box-shadow: 0 0 10px var(--candle)`): the network dot while the indexer answers; removed when offline or waiting.

### Named Rules
**The Stacked Pad Rule.** Depth means another sheet of paper. Offsets between copies are the world's own device and stay in the paper colours; do not replace them with a generic drop shadow, and do not add shadows to boxes inside the sheet.

## Shapes

Every surface takes the same 2px corner: tabs (top corners only), the sheet and under-copies (bottom corners only), chips, invoice rows, buttons, skeletons, tags and the focus ring. Circles are reserved for status dots, the stub index ring (22px) and the attacker avatar ring (34px). The only soft shapes are the 30px-radius blurred blot that stands in for a hidden figure and the scalloped wax seal. Boxes have no borders of their own; they are ruled by 1px hairlines shared with their neighbours under a strong top rule, and the accountant's check is separated by a dashed rule. Stamps are double-ruled frames with 3px and 1.5px corners, roughened by an ink displacement filter and always rotated (-8 degrees on the sheet, -6 and 5 on stubs, -7 in the attack list).

## Components

### Shell bar
A dark strip (night mixed 70% with black), 52px tall (48px on mobile), with cells divided by night-rule hairlines: the brand (crow inside a hexagon outline, 17px bold), the guided-mode note, the network state with its pulse and live epoch, and the mute toggle. The pulse beats once (scale 1.8 to 1, 280ms) on each successful indexer read.

### Copy tabs
Torn stubs of the pad. A five-column row, 4px apart, each tab in its own viewer's paper with a 22px icon, the viewer name (650, 15px) and a one-line role ("Checks the books"). Unselected tabs sit 6px lower and dimmed with a perforated bottom edge; hover lifts to 2px and clears the dim, press goes to 4px with scale 0.99, the selected tab sits flush and joins the sheet. Transitions 160ms. Arrow keys, Home and End move between them. The focus ring on a tab is carbon, inset 4px.

### The crow
An inline SVG crow (84 by 63px, 52 by 39px on mobile) perched on the top edge of the tab row at the right, feet on the last stub, ignoring the pointer. Its body is near-black night ink with a bone rim and a candle eye. Every 11s (starting after 2s) it pecks: a short rotation to -7, 2 and -3 degrees over the last 14% of the loop on the move easing. The same crow, in bone with a night eye, sits inside the brand mark.

### Ruled box (form field)
Padding 12px 16px 14px (10px 14px 12px on mobile). A 12px semibold label with an icon, then the value. The status box adds a line saying where the fact came from: a green dot and "Read live from the book", or a red dot and "Recorded status, indexer unreachable". The first box under the title is a caption, lightly tinted with carbon, that says in 17px what this copy is.

### Hidden box (not on this copy)
A box whose data did not transfer keeps its label, fills with an SVG carbon smudge stretched to the box that drifts slowly (9s, opacity 0.55 to 0.38 and 6px sideways), and shows a "Not on this copy" chip with a lock icon plus screen-reader text naming what is hidden. A hidden amount draws a fixed 150 by 58px blot (30px radius, 9px blur, carbon at 22%) next to the unit.

### Stamp
Inline SVG over the form's top edge, rotated -8 degrees, up to 200px wide and shrinking with the sheet so it never covers the invoice number. Red for SETTLED and REFUNDED, carbon at 0.8 for ISSUED, FUNDED and APPROVED. When the copy or invoice changes it lands from scale 1.04 through 0.95 to 1 (220ms, 140ms delay). Mini stamps (84px) mark each stub COMMITTED (carbon) or REFUSED (red).

### Wax seal
A 56px scalloped wax disc in stamp red with a darker rim and an embossed mark, set at the left of the escrow box. Once the escrow output is spent the wax fades to 55% red in the paper and a crack in the paper colour runs across it. The change is a state swap, not an animation.

### Action chips
Inline chips with icon, action, state and, once done, a link to its transaction. Available: 1px strong rule on paper lightened toward Original. Not available: dashed rule, transparent, secondary ink.

### Rules list
A two-column list (one column on mobile) of the escrow's spending paths, each with a 20px icon, the path name, its condition, a carbon "yours" tag for the current viewer's path and an outlined "used" tag for the path that spent the escrow.

### Invoice rows (navigation)
Full-width buttons on Original mixed 84% with night; hover goes to full Original, press scales to 0.98 (150ms), the current one gets the candle ring and `aria-current`. A funded invoice's hourglass turns half a revolution every 6s.

### Transaction stubs
A tear-off list on Original paper with perforations between stubs: a numbered ring, what happened, who signed an attack, the rejection reason in red ink, a transaction link and fee in µT, the live verdict with its icon, and the mini stamp. A "Check again on chain" link-button in bone sits above the list.

### Try to steal it
A ruled list on the desk, one row per attack: a 34px ring with the signer's icon, the attack title and signer, the guard that refused it with its icon, and a "Try it" button (night-2 fill, strong night rule, 650, 13px). Pressing it asks the indexer, then hides the button, prints "Refused by the chain" with the reason and source, slams a REFUSED stamp onto the row (scale 1.25 to 0.96 to 1, 220ms) and plays the stamp sound. A 56px tally counts attempts, and "Replay them all" (outlined in bone-3) fires the rows 320ms apart.

### Skeleton
A 2px-radius bar of the current text colour at 16%, pulsing to 0.45 opacity every 1.6s, with `role="status"` and a label saying what is being read. No default value is ever painted while a fact loads.

### Buttons on the desk
Transparent or night-2 fill, 1px night-rule-strong or bone-3 border, bone text at 600 to 650, 2px corners. Hover moves the fill to night-2 or night-3 (150ms); press scales to 0.97 (120ms). Disabled drops to 0.5 to 0.6 opacity.

### Motion and sound
Easings: out `cubic-bezier(.23,1,.32,1)`, move `cubic-bezier(.77,0,.175,1)`. Switching copy or invoice: the old sheet slides 56px down, tilts 0.8 degrees and fades (240ms, move), the new one rises from 18px (240ms, out), and the stamp lands (220ms, 140ms delay). Entrance: blocks rise 6px over 260ms in 60ms steps; stubs over 240ms after 240ms with 30ms per row; attack rows over 240ms after 300ms with 30ms per row; row index capped at 8. Stubs play their entrance once per invoice. Interaction transitions run 120 to 160ms. Ambient loops: skeleton pulse 1.6s, smudge drift 9s, funded hourglass 6s, crow peck 11s. `prefers-reduced-motion: reduce` removes every animation and transition and skips the sheet slide and the replay delay.

Three Kenney CC0 clips at volume 0.28, played only from a user gesture and silenced by a mute toggle in the bar remembered in local storage: `switch.ogg` when the visitor changes copy, `stamp.ogg` when the visitor opens another invoice or an attack is refused, `verified.ogg` when a visitor-triggered recheck confirms every verdict.

## Do's and Don'ts

### Do:
- **Do** give every viewer its own paper (Public Original, Client Canario, Contractor Rosa, Mediator Salvia, Accountant Azul) and derive the sheet from `--copy`.
- **Do** keep paper light and the desk dark: carbon ink on paper, bone ink on night, and the night form of a status colour (sello-night, verde-night, candle) whenever it sits on the desk.
- **Do** draw hidden values as value-independent smudges: a smudge stretched to the box and a fixed 150 by 58px blot, plus the "Not on this copy" chip with its lock and screen-reader text.
- **Do** read status, epoch and verdicts live from the indexer, show a labelled skeleton while reading, and on failure fall back to the recorded run with an explicit "recorded" or "indexer unreachable" note and the red dot.
- **Do** put a Phosphor Light icon on box labels, stub verdicts, actions, attack rows, section headings and table rows, at one stroke weight.
- **Do** keep movement on transform and opacity under 300ms, with the reduced-motion override covering every animation and transition.
- **Do** use the 2px corner on every rectangular surface.

### Don't:
- **Don't** stamp anything that is not a verdict, and don't use stamp red for decoration, fills or headings.
- **Don't** size, shape or blur a hidden value from its real length or magnitude; the smudge must look the same for 1 tTARI and 1,000,000.
- **Don't** paint a default value (0, "Issued", an empty hash) while the indexer is answering.
- **Don't** split the form into floating cards with their own shadows and radii; it stays one ruled sheet on a stacked pad.
- **Don't** use pure black or pure white; the desk is night and the lightest paper is Original.
- **Don't** introduce a second type family or uppercase text outside the stamps.
- **Don't** play a sound without a user gesture or with the toggle off.
