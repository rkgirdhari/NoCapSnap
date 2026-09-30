<script lang="ts">
  import { onMount } from "svelte";
  import { bridge, type AppInfo, type MenuItem, type Profile, type RemoteLocation, type StoreStatus } from "$lib/bridge";
  import TopBar from "$lib/components/TopBar.svelte";
  import { cameraRoute, setCameraRoute, type CameraRoute } from "$lib/prefs";

  let info = $state<AppInfo | null>(null);
  let profile = $state<Profile | null>(null);
  let menu = $state<MenuItem[]>([]);
  let status = $state<StoreStatus | null>(null);
  let name = $state("");
  let nameNote = $state<string | null>(null);
  let route = $state<CameraRoute>(cameraRoute());
  let check = $state<{ ok: boolean; text: string } | null>(null);
  let checking = $state(false);
  let ackNote = $state<string | null>(null);
  let error = $state<string | null>(null);

  // Sign-in (W3a). The password goes straight to Rust and is never stored.
  let serverUrl = $state("");
  let login = $state("");
  let password = $state("");
  let signingIn = $state(false);
  let sessionNote = $state<string | null>(null);
  let locations = $state<RemoteLocation[]>([]);
  let syncing = $state(false);

  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

  onMount(async () => {
    try {
      [info, profile, menu, status] = await Promise.all([
        bridge.appInfo(),
        bridge.profile(),
        bridge.menu(),
        bridge.status(),
      ]);
      name = profile.displayName ?? "";
      serverUrl = profile.serverUrl ?? "";
      if (profile.signedIn) {
        const session = await bridge.refreshSession().catch(() => null);
        if (session) {
          profile = session.profile;
          locations = session.locations;
          menu = await bridge.menu();
        } else {
          profile = await bridge.profile(); // the server may have signed this phone out
        }
      }
    } catch (e) {
      error = message(e);
    }
  });

  async function signIn(event: SubmitEvent) {
    event.preventDefault();
    signingIn = true;
    sessionNote = null;
    try {
      const session = await bridge.signIn(serverUrl, login, password);
      profile = session.profile;
      locations = session.locations;
      name = profile.displayName ?? "";
      menu = await bridge.menu();
      password = "";
      sessionNote = "Signed in. Plates now sync to your restaurant.";
    } catch (e) {
      sessionNote = message(e);
    } finally {
      signingIn = false;
    }
  }

  async function signOut() {
    sessionNote = null;
    try {
      profile = await bridge.signOut();
      locations = [];
      menu = await bridge.menu();
      sessionNote = "Signed out. Plates already on this phone stay here.";
    } catch (e) {
      sessionNote = message(e);
    }
  }

  async function pickLocation(event: Event) {
    const id = (event.currentTarget as HTMLSelectElement).value;
    try {
      profile = await bridge.chooseLocation(id);
      menu = await bridge.menu();
    } catch (e) {
      sessionNote = message(e);
    }
  }

  async function syncNow() {
    syncing = true;
    sessionNote = null;
    try {
      const r = await bridge.syncNow();
      status = await bridge.status();
      sessionNote = r.stopped ? `Not synced: ${r.stopped}.` : `${r.synced} synced, ${r.remaining} waiting.`;
    } catch (e) {
      sessionNote = message(e);
    } finally {
      syncing = false;
    }
  }

  async function saveName(event: SubmitEvent) {
    event.preventDefault();
    nameNote = null;
    try {
      profile = await bridge.setDisplayName(name);
      name = profile.displayName ?? "";
      nameNote = "Saved on this device.";
    } catch (e) {
      nameNote = message(e);
    }
  }

  function chooseRoute(next: CameraRoute) {
    route = next;
    setCameraRoute(next);
  }

  async function runCheck() {
    checking = true;
    check = null;
    try {
      const text = await bridge.selftest();
      status = await bridge.status();
      check = { ok: true, text };
    } catch (e) {
      check = { ok: false, text: message(e) };
    } finally {
      checking = false;
    }
  }

  async function simulateAck() {
    ackNote = null;
    try {
      const acked = await bridge.simulateAck();
      status = await bridge.status();
      ackNote = acked ? `Marked synced: ${acked.dishName ?? "unlabelled dish"}.` : "Nothing is waiting.";
    } catch (e) {
      ackNote = message(e);
    }
  }
</script>

<TopBar status />

<main class="page settings">
  <h1 class="display">Settings.</h1>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <section>
    <h2 class="kicker">You</h2>
    <form class="card pad" onsubmit={saveName}>
      <label class="field">
        <span>Name for the greeting</span>
        <input type="text" maxlength="40" autocomplete="given-name" bind:value={name} placeholder="e.g. Maya" />
      </label>
      <button class="btn outline" type="submit">Save name</button>
      <p class="note muted">{nameNote ?? "Stored on this device only."}</p>
    </form>
  </section>

  <section>
    <h2 class="kicker">Restaurant</h2>
    {#if profile?.signedIn}
      <div class="card pad">
        <p class="big">{profile.organizationName}</p>
        <p class="muted">
          Signed in as {profile.displayName}{profile.role ? ` (${profile.role})` : ""}. {menu.length} dishes on the menu.
        </p>
        {#if locations.length > 1}
          <label class="field">
            <span>Location</span>
            <select value={profile.locationId} onchange={pickLocation}>
              {#each locations as l (l.id)}<option value={l.id}>{l.name}</option>{/each}
            </select>
          </label>
        {:else}
          <p class="muted">Location: {profile.locationName}</p>
        {/if}
        <div class="row">
          <button class="btn outline" onclick={syncNow} disabled={syncing}>{syncing ? "Syncing…" : "Sync now"}</button>
          <button class="btn outline" onclick={signOut}>Sign out</button>
        </div>
        <p class="note muted mono">{profile.serverUrl}</p>
      </div>
    {:else}
      <div class="card pad">
        <p class="big">{profile?.locationName ?? "No location"}<span class="demo">Demo</span></p>
        <p class="muted">Sample location and menu until this phone signs in to your restaurant. Demo plates stay on this phone.</p>
      </div>
      <form class="card pad signin" onsubmit={signIn}>
        <label class="field">
          <span>Server address</span>
          <input type="url" inputmode="url" autocomplete="url" autocapitalize="none" spellcheck="false"
            placeholder="https://capsnap.example.com" bind:value={serverUrl} required />
        </label>
        <label class="field">
          <span>Sign-in name</span>
          <input type="text" autocomplete="username" autocapitalize="none" spellcheck="false" bind:value={login} required />
        </label>
        <label class="field">
          <span>Password</span>
          <input type="password" autocomplete="current-password" bind:value={password} required />
        </label>
        <button class="btn primary" type="submit" disabled={signingIn}>{signingIn ? "Signing in…" : "Sign in"}</button>
      </form>
    {/if}
    {#if sessionNote}<p class="note muted" role="status">{sessionNote}</p>{/if}
  </section>

  <section>
    <h2 class="kicker">Camera</h2>
    <div class="card pad" role="radiogroup" aria-label="Camera">
      <label class="opt">
        <input type="radio" name="route" checked={route === "viewfinder"} onchange={() => chooseRoute("viewfinder")} />
        <span><strong>In-app viewfinder</strong><span class="muted">Frame the plate without leaving CapSnap.</span></span>
      </label>
      <label class="opt">
        <input type="radio" name="route" checked={route === "phone"} onchange={() => chooseRoute("phone")} />
        <span><strong>Phone camera app</strong><span class="muted">Your camera app, then back to CapSnap.</span></span>
      </label>
    </div>
  </section>

  <section>
    <h2 class="kicker">Privacy</h2>
    <div class="card pad">
      <p class="muted">
        Before a photo is saved, this phone resizes it to 2048 px and removes its location and camera data. No guest
        names, emails or phone numbers are ever collected.
      </p>
    </div>
  </section>

  <section>
    <h2 class="kicker">Device check</h2>
    <div class="card pad">
      <button class="btn outline" onclick={runCheck} disabled={checking}>
        {checking ? "Checking…" : "Run device check"}
      </button>
      {#if check}
        <p class="note mono" class:bad={!check.ok} role="status">{check.text}</p>
      {/if}
      {#if status}
        <dl class="facts">
          <dt>Saved offline</dt><dd>{status.pending}</dd>
          <dt>Synced</dt><dd>{status.synced}</dd>
          <dt>SQLite</dt><dd class="mono">{status.sqliteVersion} · {status.journalMode}</dd>
        </dl>
      {/if}
    </div>
  </section>

  {#if info?.debug}
    <section>
      <h2 class="kicker">Developer · debug build only</h2>
      <div class="card pad">
        <p class="muted">Stands in for the server acknowledging the oldest waiting plate, to review the “QR ready” screens.</p>
        <button class="btn outline" onclick={simulateAck}>Simulate server acknowledgement</button>
        {#if ackNote}<p class="note muted" role="status">{ackNote}</p>{/if}
      </div>
    </section>
  {/if}

  <p class="helper about">
    CapSnap {info?.version ?? ""}{bridge.mode === "preview" ? " · browser preview" : ""} · Hammurabi Coding Company LLC
  </p>
</main>

<style>
  .settings .display {
    margin-top: 8px;
  }
  section {
    margin-top: 24px;
  }
  section .kicker {
    margin-bottom: 10px;
  }
  .pad {
    display: grid;
    gap: 12px;
    padding: 16px;
  }
  .pad p {
    margin: 0;
  }
  .big {
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--serif);
    font-size: 22px;
    font-weight: 700;
    color: var(--headline);
  }
  .demo {
    padding: 2px 7px;
    border-radius: 5px;
    background: var(--wait-bg);
    color: var(--wait);
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  .field {
    display: grid;
    gap: 8px;
  }
  .field input,
  .field select {
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
  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .signin {
    margin-top: 10px;
  }
  section > .note {
    margin: 10px 4px 0;
  }
  .opt {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 12px;
    align-items: start;
    min-height: 44px;
    cursor: pointer;
  }
  .opt input {
    width: 20px;
    height: 20px;
    margin: 2px 0 0;
    accent-color: var(--select);
  }
  .opt strong {
    display: block;
    font-weight: 600;
  }
  .opt .muted {
    font-size: 14.5px;
  }
  .note {
    font-size: 14px;
  }
  .note.bad {
    color: var(--danger);
  }
  .facts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 16px;
    margin: 0;
    font-size: 14.5px;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
  }
  .about {
    margin: 28px 0 0;
    text-align: center;
  }
</style>
