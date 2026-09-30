# NO CAP SNAP — owner mockups (design reference)

Received from the owner on 2026-09-30. All five are 1125×2000 WebP files. They are the design target proposed for W2
([`../../protocol/W2-PROPOSAL-staff-ui-from-mockups.md`](../../protocol/W2-PROPOSAL-staff-ui-from-mockups.md)).

| File | Screen |
|---|---|
| `01-home.webp` | Home: greeting, location, tonight's captures, "Capture a dish", sync stats, recent plates |
| `02-prepare-which-dish.webp` | Capture 01/03 Prepare: menu search, categories, dish picker, table label |
| `03-review-plate-as-served.webp` | Capture 02/03 Review: photo, dish · location · table, privacy hint, save |
| `04-history-offline.webp` | History while offline: waiting-to-sync banner, last synced, list, retry |
| `05-guest-invitation-qr.webp` | Guest invitation: synced · QR ready, concept QR (not live) |

## Palette measured from these files

Surfaces are the median of an 11×11 px patch; text is the median of the brightest (or most saturated) 6% of
pixels inside the glyph box. Contrast is WCAG 2.x. Reproduce with `python3 ../sample-mockups.py` (needs Pillow).

| Role | Hex | Measured contrast |
|---|---|---|
| Background | `#190926` | — |
| Headline cream | `#FCE9C5` | 15.88:1 on background |
| CTA yellow / its ink | `#FDCF10` / `#110803` | 13.31:1 |
| Selected border, check | `#F9CE22` | — |
| Active tab gold | `#CFB242` | 9.11:1 on background |
| Stat card, location pill | `#533A39`, `#4F3837` | card against background 1.83:1, so its border carries the edge |
| Search / input | `#402C2E` | — |
| Dish card, review card, banner | `#2A1B27`, `#291928`, `#291925` | — |
| List card, recent card | `#1E1223`, `#1F1027` | — |
| Kicker / muted label | `#98827C`, `#8C7A77` | 5.25:1, 4.66:1 on background |
| Secondary text | `#A1949B`, `#A396A1` | 6.22:1 on card, 6.71:1 on background |
| Helper text | `#8A7D86`, `#908390` | 4.83:1, 5.26:1 on background |
| Stat label | `#D0BDB7` | 5.73:1 on stat card |
| Online / QR-ready green | `#8FEB8C` (dot), `#9AD999` on `#272F28` | 8.37:1 |
| Offline / waiting amber | `#F2BF72`, `#E8B872` on `#3A2827` | 11.25:1, 7.62:1 |
| QR card / modules | `#FDF0CE` / `#1D0E25` | 16.23:1 |

Every text pair measured passes WCAG AA for normal text.
