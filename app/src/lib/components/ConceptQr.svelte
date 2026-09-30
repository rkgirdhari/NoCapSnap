<script lang="ts">
  // Decorative stand-in for the guest QR (W3 issues real invitation tokens).
  // Deliberately NOT a QR code: 23×23 modules (no QR version has that size),
  // no timing patterns, no format or version information, and dots drawn from
  // a seeded pseudo-random pattern. Scanners must find nothing here; the W2
  // report records a decoder run against a screenshot.
  let { seed }: { seed: string } = $props();

  const N = 23;
  const FINDERS = [
    [0, 0],
    [N - 7, 0],
    [0, N - 7],
  ];

  function rng(hex: string) {
    let a = parseInt(hex.slice(0, 8) || "c0ffee", 16) >>> 0;
    return () => {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  const reserved = (x: number, y: number) =>
    FINDERS.some(([fx, fy]) => x >= fx - 1 && x <= fx + 7 && y >= fy - 1 && y <= fy + 7) ||
    (Math.abs(x - (N - 1) / 2) <= 2.5 && Math.abs(y - (N - 1) / 2) <= 2.5);

  const dots = $derived.by(() => {
    const next = rng(seed);
    const out: [number, number][] = [];
    for (let y = 0; y < N; y++)
      for (let x = 0; x < N; x++) if (!reserved(x, y) && next() < 0.46) out.push([x, y]);
    return out;
  });
</script>

<svg viewBox="-2 -2 {N + 4} {N + 4}" role="img" aria-label="Concept guest QR, not live">
  {#each FINDERS as [fx, fy] (`${fx}-${fy}`)}
    <rect x={fx + 0.5} y={fy + 0.5} width="6" height="6" rx="1.8" class="ring" />
    <rect x={fx + 2} y={fy + 2} width="3" height="3" rx="0.7" class="ink" />
  {/each}
  {#each dots as [x, y] (`${x}-${y}`)}
    <circle cx={x + 0.5} cy={y + 0.5} r="0.43" class="ink" />
  {/each}
  <!-- A four-point star where a logo would sit. -->
  <path
    class="ink"
    d="M11.5 8.6 C11.9 10.6 12.4 11.1 14.4 11.5 C12.4 11.9 11.9 12.4 11.5 14.4 C11.1 12.4 10.6 11.9 8.6 11.5 C10.6 11.1 11.1 10.6 11.5 8.6 Z"
  />
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: auto;
  }
  .ink {
    fill: var(--qr-ink);
  }
  .ring {
    fill: none;
    stroke: var(--qr-ink);
    stroke-width: 1;
  }
</style>
