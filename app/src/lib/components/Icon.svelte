<script lang="ts" module>
  export type IconName =
    | "home"
    | "history"
    | "insights"
    | "settings"
    | "sync"
    | "cloud-off"
    | "camera"
    | "chevron-right"
    | "chevron-down"
    | "arrow-left"
    | "search"
    | "check"
    | "hourglass"
    | "qr"
    | "database"
    | "stack"
    | "save"
    | "info"
    | "plate";
</script>

<script lang="ts">
  // Drawn for CapSnap on a 24-unit grid; stroke icons that inherit the text colour.
  let {
    name,
    size = 22,
    stroke = 1.6,
    filled = false,
  }: { name: IconName; size?: number; stroke?: number; filled?: boolean } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "home"}
    <path d="M3.5 11 12 4l8.5 7" /><path
      d="M5.5 9.5V20h4.5v-5.5h4V20h4.5V9.5"
      fill={filled ? "currentColor" : "none"}
    />
  {:else if name === "history"}
    <circle cx="12" cy="12" r="8.5" /><path d="M12 7.5V12l3 2" />
  {:else if name === "insights"}
    <path d="M6 20v-8" /><path d="M10 20V5" /><path d="M14 20V8" /><path d="M18 20v-5" />
  {:else if name === "settings"}
    <circle cx="12" cy="12" r="6" /><circle cx="12" cy="12" r="2.4" />
    <path
      d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1"
    />
  {:else if name === "sync"}
    <path d="M19.5 10.5A7.8 7.8 0 0 0 6 6.8L4.5 8.5" /><path d="M4.5 4.5v4h4" />
    <path d="M4.5 13.5A7.8 7.8 0 0 0 18 17.2l1.5-1.7" /><path d="M19.5 19.5v-4h-4" />
  {:else if name === "cloud-off"}
    <path d="M8.5 18.5H7a4.5 4.5 0 0 1-.9-8.9A6.2 6.2 0 0 1 17.6 8.6a4.9 4.9 0 0 1 1.9 9.1" /><path
      d="m3.5 3.5 17 17"
    />
  {:else if name === "camera"}
    <path d="M3.5 8.5a1.5 1.5 0 0 1 1.5-1.5h2.6l1.6-2.5h5.6l1.6 2.5H19a1.5 1.5 0 0 1 1.5 1.5v10H3.5z" />
    <circle cx="12" cy="13.2" r="3.6" />
  {:else if name === "chevron-right"}
    <path d="m9.5 5.5 6.5 6.5-6.5 6.5" />
  {:else if name === "chevron-down"}
    <path d="m6 9.5 6 6 6-6" />
  {:else if name === "arrow-left"}
    <path d="M20 12H4.5" /><path d="m10.5 5.5-6.5 6.5 6.5 6.5" />
  {:else if name === "search"}
    <circle cx="10.5" cy="10.5" r="6.5" /><path d="m15.5 15.5 5 5" />
  {:else if name === "check"}
    <path d="m5 12.5 4.2 4.2L19 7" />
  {:else if name === "hourglass"}
    <path d="M6.5 3.5h11M6.5 20.5h11" />
    <path d="M8 3.5v2.3c0 2.4 4 3.9 4 6.2s-4 3.8-4 6.2v2.3M16 3.5v2.3c0 2.4-4 3.9-4 6.2s4 3.8 4 6.2v2.3" />
  {:else if name === "qr"}
    <rect x="4" y="4" width="6.5" height="6.5" rx="1.2" /><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.2" />
    <rect x="4" y="13.5" width="6.5" height="6.5" rx="1.2" />
    <path d="M13.5 13.5h2.5v2.5M20 13.5v0M18 18h2v2M13.5 20h2" />
  {:else if name === "database"}
    <ellipse cx="12" cy="6" rx="7" ry="2.8" /><path d="M5 6v12c0 1.5 3.1 2.8 7 2.8s7-1.3 7-2.8V6" />
    <path d="M5 12c0 1.5 3.1 2.8 7 2.8s7-1.3 7-2.8" />
  {:else if name === "stack"}
    <rect x="4" y="4.5" width="16" height="4.5" rx="2.2" /><rect x="4" y="11" width="16" height="4.5" rx="2.2" />
    <path d="M4 19.5h16" />
  {:else if name === "save"}
    <path d="M5 4h11.5L19.5 7v12.5a.5.5 0 0 1-.5.5H5a.5.5 0 0 1-.5-.5v-15A.5.5 0 0 1 5 4z" />
    <path d="M8 4v5h7V4" /><rect x="8" y="13.5" width="8" height="6.5" rx=".5" />
  {:else if name === "info"}
    <circle cx="12" cy="12" r="8.5" /><path d="M12 11v5.5" /><path d="M12 7.6v.1" />
  {:else if name === "plate"}
    <circle cx="12" cy="12" r="8.5" /><circle cx="12" cy="12" r="5" />
  {/if}
</svg>
