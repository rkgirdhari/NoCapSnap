<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { bridge } from "$lib/bridge";
  import Icon, { type IconName } from "$lib/components/Icon.svelte";

  let { children } = $props();

  // Lets app.css reserve room for Android's status and gesture bars (device only).
  if (typeof document !== "undefined") document.documentElement.dataset.shell = bridge.mode;

  const tabs: { href: string; label: string; icon: IconName }[] = [
    { href: "/", label: "Home", icon: "home" },
    { href: "/history", label: "History", icon: "history" },
    { href: "/insights", label: "Insights", icon: "insights" },
    { href: "/settings", label: "Settings", icon: "settings" },
  ];

  // Capture belongs to Home. The camera, review and invitation screens take
  // the whole screen, as in the mockups.
  const section = $derived.by(() => {
    const path = page.url.pathname;
    if (path === "/" || path.startsWith("/capture")) return "/";
    return tabs.find((t) => t.href !== "/" && path.startsWith(t.href))?.href ?? null;
  });
  const showTabs = $derived(
    section !== null && !(page.url.pathname.startsWith("/capture") && page.state.step),
  );
</script>

<div class="shell" class:with-tabs={showTabs}>
  {#if bridge.mode === "preview"}
    <p class="ribbon">Browser preview · nothing is saved</p>
  {/if}

  {#if bridge.mode === "none"}
    <main class="page none">
      <p class="kicker">NO CAP SNAP</p>
      <h1 class="display">Open CapSnap on the phone.</h1>
      <p class="muted">This page is the staff app's shell. It only works inside the CapSnap Android app.</p>
    </main>
  {:else}
    {@render children()}
  {/if}

  {#if showTabs}
    <nav class="tabbar" aria-label="Sections">
      {#each tabs as tab (tab.href)}
        {@const active = section === tab.href}
        <a class="tab" class:active href={tab.href} aria-current={active ? "page" : undefined}>
          <Icon name={tab.icon} size={24} stroke={1.5} filled={active && tab.icon === "home"} />
          <span>{tab.label}</span>
        </a>
      {/each}
    </nav>
  {/if}
</div>

<style>
  .shell {
    min-height: 100dvh;
  }
  .shell.with-tabs {
    padding-bottom: calc(var(--tabbar) + var(--safe-bottom));
  }
  /* Full-screen steps (camera, review, invitation) still end above Android's navigation bar. */
  .shell:not(.with-tabs) {
    padding-bottom: var(--safe-bottom);
  }

  .ribbon {
    margin: 0;
    padding: calc(4px + var(--safe-top)) 12px 4px;
    background: var(--wait-bg);
    color: var(--wait);
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-align: center;
    text-transform: uppercase;
  }

  .none {
    padding-top: 20vh;
  }
  .none > * + * {
    margin-top: 16px;
  }

  .tabbar {
    position: fixed;
    inset: auto 0 0 0;
    z-index: 10;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    height: calc(var(--tabbar) + var(--safe-bottom));
    padding-bottom: var(--safe-bottom);
    background: color-mix(in srgb, var(--bg-bar) 94%, transparent);
    backdrop-filter: blur(12px);
  }

  .tab {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    color: var(--tab-idle);
    font-size: 12.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    text-decoration: none;
  }

  .tab.active {
    color: var(--tab-active);
    font-weight: 650;
  }

  .tab.active::after {
    content: "";
    position: absolute;
    bottom: 6px;
    width: 36px;
    height: 3px;
    border-radius: 2px;
    background: var(--tab-active);
  }
</style>
