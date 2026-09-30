import { useRef, useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { CameraView, useCameraPermissions } from "expo-camera";
import { router } from "expo-router";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { setPendingCapture } from "@/state/pendingCapture";

export default function CaptureScreen() {
  const cameraRef = useRef<CameraView>(null);
  const [permission, requestPermission] = useCameraPermissions();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const insets = useSafeAreaInsets();

  if (!permission) return <View style={styles.wrap} />;
  if (!permission.granted) {
    return (
      <View style={styles.wrap}>
        <Text style={styles.copy}>Camera access is required to snap the dish.</Text>
        <Pressable style={styles.btn} onPress={requestPermission}>
          <Text style={styles.btnText}>Grant camera</Text>
        </Pressable>
      </View>
    );
  }

  async function snap() {
    if (!cameraRef.current || busy) return;
    setBusy(true);
    setError(null);
    try {
      const photo = await cameraRef.current.takePictureAsync({
        quality: 0.85,
        base64: true,
      });
      if (!photo.base64) throw new Error("Camera returned no image data");
      // The base64 payload is several MB — keep it out of route params.
      setPendingCapture({ uri: photo.uri, base64: photo.base64 });
      router.push("/preview");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Capture failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <View style={styles.wrap}>
      <CameraView ref={cameraRef} style={styles.camera} facing="back" />
      <View style={[styles.dock, { paddingBottom: 20 + insets.bottom }]}>
        <Text style={styles.hint}>STUB: dish selector + table # go here</Text>
        {error ? <Text style={styles.error}>{error}</Text> : null}
        <Pressable style={styles.shutter} onPress={snap} disabled={busy}>
          <Text style={styles.shutterText}>{busy ? "…" : "SNAP"}</Text>
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { flex: 1, backgroundColor: "#000" },
  camera: { flex: 1 },
  copy: { color: "#fff", padding: 24 },
  dock: { padding: 20, backgroundColor: "#111", alignItems: "center" },
  hint: { color: "#888", marginBottom: 12 },
  error: { color: "#FF6B6B", marginBottom: 12 },
  shutter: {
    width: 84,
    height: 84,
    borderRadius: 42,
    backgroundColor: "#C8F5C0",
    alignItems: "center",
    justifyContent: "center",
  },
  shutterText: { fontWeight: "800", color: "#0B0F0C" },
  btn: { backgroundColor: "#C8F5C0", margin: 24, padding: 14, borderRadius: 10 },
  btnText: { textAlign: "center", fontWeight: "700" },
});
