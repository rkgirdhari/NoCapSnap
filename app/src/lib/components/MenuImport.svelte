<script lang="ts">
  import { bridge, type ImportDraft, type MenuItem } from "$lib/bridge";

  // Paste a restaurant's own website, check the dishes found, save them as the menu (ONB-1).
  // The server reads the site; nothing is saved until the owner presses save. Dishes have no
  // photos here: each shows the plate placeholder until a plate is captured.
  let { currentCount, onSaved }: { currentCount: number; onSaved: (menu: MenuItem[]) => void } = $props();

  // Word for word what the server expects (capsnap-sync's IMPORT_CONSENT; a test keeps them equal).
  const CONSENT =
    "I own or manage this website and allow NO CAP SNAP to read its public pages for this setup.";

  let siteUrl = $state("");
  let agreed = $state(false);
  let reading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let done = $state<string | null>(null);
  let draft = $state<ImportDraft | null>(null);
  // One row per dish found, so the owner can untick or rename before saving.
  let rows = $state<{ name: string; category: string; price: string | null; keep: boolean }[]>([]);

  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
  const kept = $derived(rows.filter((r) => r.keep && r.name.trim()));
  const groups = $derived.by(() => {
    const out: { category: string; rows: typeof rows }[] = [];
    for (const r of rows) {
      let g = out.find((x) => x.category === r.category);
      if (!g) out.push((g = { category: r.category, rows: [] }));
      g.rows.push(r);
    }
    return out;
  });

  async function read(event: SubmitEvent) {
    event.preventDefault();
    reading = true;
    error = null;
    done = null;
    draft = null;
    rows = [];
    try {
      const found = await bridge.previewMenuImport(siteUrl.trim());
      draft = found;
      rows = found.menu.map((d) => ({ name: d.name, category: d.category, price: d.price, keep: true }));
    } catch (e) {
      error = message(e);
    } finally {
      reading = false;
    }
  }

  async function save() {
    saving = true;
    error = null;
    try {
      const menu = await bridge.saveMenuImport(kept.map((r) => ({ name: r.name.trim(), category: r.category })));
      done = `Saved ${menu.length} dishes as this restaurant's menu.`;
      draft = null;
      rows = [];
      siteUrl = "";
      agreed = false;
      onSaved(menu);
    } catch (e) {
      error = message(e);
    } finally {
      saving = false;
    }
  }

  function discard() {
    draft = null;
    rows = [];
  }
</script>

<div class="card pad import">
  <p class="big">Menu from a website</p>
  <p class="muted">Paste the restaurant's own website. CapSnap finds the dishes and you check them before they're saved.</p>

  <form onsubmit={read}>
    <label class="field">
      <span>Restaurant website</span>
      <input type="url" inputmode="url" autocapitalize="none" spellcheck="false" autocomplete="url"
        placeholder="https://yourrestaurant.com" bind:value={siteUrl} required />
    </label>
    <label class="consent">
      <input type="checkbox" bind:checked={agreed} />
      <span>{CONSENT}</span>
    </label>
    <button class="btn outline" type="submit" disabled={reading || !agreed || !siteUrl.trim()}>
      {reading ? "Reading the website…" : "Read the menu"}
    </button>
  </form>

  {#if error}<p class="note bad" role="alert">{error}</p>{/if}
  {#if done}<p class="note ok" role="status">{done}</p>{/if}

  {#if draft}
    <div class="review">
      <p class="muted">
        {#if draft.businessName}<strong>{draft.businessName}</strong> · {/if}
        {rows.length ? `${rows.length} dishes found. Untick any that shouldn't be on the menu, or fix a name.` : "No dishes found."}
      </p>
      {#each groups as g (g.category)}
        <h3 class="course">{g.category}</h3>
        {#each g.rows as r, i (g.category + i)}
          <div class="dish" class:off={!r.keep}>
            <input type="checkbox" bind:checked={r.keep} aria-label={`Keep ${r.name}`} />
            <input class="name" type="text" maxlength="120" bind:value={r.name} aria-label="Dish name" disabled={!r.keep} />
            {#if r.price}<span class="price">{r.price}</span>{/if}
          </div>
        {/each}
      {/each}
      {#each draft.notes as n (n)}<p class="note muted">{n}</p>{/each}

      {#if rows.length}
        <p class="note muted">
          {currentCount
            ? `This replaces the ${currentCount} dishes on the menu now. Plates already taken keep their dish name.`
            : "These become this restaurant's menu."}
        </p>
        <div class="actions">
          <button class="btn primary" onclick={save} disabled={saving || kept.length === 0}>
            {saving ? "Saving…" : `Save ${kept.length} dishes`}
          </button>
          <button class="btn outline" onclick={discard} disabled={saving}>Discard</button>
        </div>
      {:else}
        <button class="btn outline" onclick={discard}>Close</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .import form {
    display: grid;
    gap: 12px;
  }
  .field {
    display: grid;
    gap: 8px;
  }
  .field input[type="url"] {
    min-height: var(--tap);
    padding: 0 14px;
    border-radius: var(--radius);
    background: var(--input);
    border: 1px solid var(--edge);
    font-size: 17px;
  }
  .field input::placeholder {
    color: var(--on-input);
  }
  .consent {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 12px;
    align-items: start;
    font-size: 14.5px;
    cursor: pointer;
  }
  .consent input {
    width: 20px;
    height: 20px;
    margin: 2px 0 0;
    accent-color: var(--select);
  }
  .review {
    display: grid;
    gap: 8px;
    margin-top: 6px;
  }
  .course {
    margin: 10px 0 0;
    font-size: 12px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .dish {
    display: grid;
    grid-template-columns: 24px 1fr auto;
    gap: 10px;
    align-items: center;
  }
  .dish input[type="checkbox"] {
    width: 20px;
    height: 20px;
    accent-color: var(--select);
  }
  .dish .name {
    min-height: 44px;
    padding: 0 12px;
    border-radius: var(--radius);
    background: var(--input);
    border: 1px solid var(--edge);
    font-size: 16px;
  }
  .dish.off .name {
    opacity: 0.5;
    text-decoration: line-through;
  }
  .price {
    color: var(--muted);
    font-size: 14px;
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .note {
    font-size: 14px;
  }
  .note.bad {
    color: var(--danger);
  }
  .note.ok {
    color: var(--ok);
  }
  .big {
    font-family: var(--serif);
    font-size: 22px;
    font-weight: 700;
    color: var(--headline);
  }
  .pad {
    display: grid;
    gap: 12px;
    padding: 16px;
  }
  .pad p {
    margin: 0;
  }
</style>
