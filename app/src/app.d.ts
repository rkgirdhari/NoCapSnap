// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
  namespace App {
    /** Shallow-routed steps inside /capture, so Android's back button walks them. */
    interface PageState {
      step?: "camera" | "review";
    }
  }
}

export {};
