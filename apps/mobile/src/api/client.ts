import { Platform } from "react-native";
import type { ApiError } from "@nocapsnap/shared";

// The Android emulator reaches the host machine at 10.0.2.2, not localhost.
const API_URL =
  process.env.EXPO_PUBLIC_API_URL ??
  (Platform.OS === "android" ? "http://10.0.2.2:3000" : "http://localhost:3000");

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
  if (!res.ok) {
    const body = await res.text();
    let message = body || `HTTP ${res.status}`;
    try {
      message = (JSON.parse(body) as ApiError).error ?? message;
    } catch {
      // non-JSON error body — use it as-is
    }
    throw new Error(message);
  }
  return res.json() as Promise<T>;
}
