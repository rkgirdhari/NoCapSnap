# M1 — Menu from a website

Date: 2026-10-07 · Status: **Built and tested on the host; not run on a phone; CI not yet run on the branch.**
The owner asked for a field where they paste a restaurant's web address and get its menu, parsed and sorted, with
placeholders where there is no photo, "tested on something like Dave & Buster's".

## Gate report

```
## Gate M1 — Menu from a website
Status: Built (host-tested); on-device use Specified
Evidence: onboard 23 tests (6 unit, 17 integration; 6 of them new), server 37 (3 new, menu_replace), sync 11 (5 new, menu_import);
          clippy -D warnings and cargo fmt clean; svelte-check 0 errors; browser test of the Settings screen
          (hidden when signed out and for the server role; consent gate; edit, untick, save; capture screen lists
          exactly the saved dishes). 5 mutations of the reader each made a test fail.
Changes: onboard (plain.rs, courses.rs, extract.rs), server PUT /locations/{id}/menu-items,
         sync client (import_preview, save_menu), Tauri commands, MenuImport.svelte in Settings
Tenth Man: see below
Decision needed from owner: none to merge; try it on the phone once a restaurant is signed in
```

## What it does

| Piece | Label | Notes |
|---|---|---|
| Paste a web address in Settings → Menu (admin/manager only) | Built | Same consent as ONB-1: the person must tick that they own or manage the site. Own sites only; no review, delivery or social sites |
| Read the page: structured menu data first, then plain text, then headings with no prices | Built | Draft only; every dish is shown to be edited or unticked; the draft says when dishes came from plain text |
| Sort into the order a diner reads a menu | Built | Starters, mains, sides, desserts, drinks; the site's own order kept inside a course |
| Save as the restaurant's menu | Built | Replaces the current menu; retired dishes stay on old captures. 1–500 dishes, tidied and de-duplicated |
| Placeholders where there is no photo | Built (already) | The dish card shows the plate icon when it has no photo; this feature never fetches photos |
| Menus drawn by JavaScript, images or PDFs | Not supported | The draft says no menu was found |
| QR code as the input | Aspirational | Not built; the address field is the input |

## What was found on real sites

- **Dave & Buster's** (the owner's example): the menu is drawn in the browser, so there is nothing in the page we
  fetch. The importer correctly reports no menu and the owner adds dishes by hand. Chains often work this way.
- **Two real pages were fetched with curl and read through the importer on a local pretend site** (so the live
  DNS, robots.txt and address checks were not exercised on those hosts):
  - Joe's Stone Crab: 5 dishes from the page's structured data. Its prices are written `$ 129.95` with a space,
    which the text reader did not handle; fixed.
  - Peter Luger: 32 dishes in four sections with no prices, dishes separated by `<br>` inside one paragraph. The
    text reader saw each paragraph as one line; fixed, and a reader for menus without prices was added. One line
    of the section blurb is read as a dish ("Family selected") and a burger line is dropped; the owner reviews both.
  - A check for "not food" words matched part of a word ("coffee" contains "fee"); fixed to whole words.
  - Each fix has a regression test, and each was broken on purpose to confirm a test fails.

## Tenth Man

- **"It works on restaurant sites" is shown on fixtures and two real pages, not on a sample.** Many sites will be
  unreadable (scripts, images, PDFs), or read with mistakes. The screen is built so that is survivable: nothing is
  saved until the owner reviews, and the draft says when dishes came from text.
- **Saving replaces the whole menu.** An owner who pastes the wrong site loses the current list (old captures keep
  their dish names). The screen says so before saving; there is no undo.
- **The consent tick is a statement by the user.** The server records it, but nothing proves they own the site.
  That is the ONB-1 position (owner decision O1) and is unchanged.
- **Not run on a phone.** The Tauri commands compile and pass clippy, and the screen passes in a browser preview;
  the Android app with this feature has not been installed.
