import { useState } from "react";
import { Pressable, StyleSheet, Text, TextInput, View } from "react-native";
import { capturePhotoStub } from "@/api/photos";
import { clearPendingCapture, getPendingCapture } from "@/state/pendingCapture";

export default function SendScreen() {
  const [table, setTable] = useState("");
  const [status, setStatus] = useState("Ready to send");
  const [sending, setSending] = useState(false);

  async function send() {
    const capture = getPendingCapture();
    if (!capture) {
      setStatus("No photo to send — snap the plate first.");
      return;
    }
    setSending(true);
    setStatus("Uploading…");
    try {
      await capturePhotoStub({
        locationId: "00000000-0000-0000-0000-000000000001",
        menuItemId: "00000000-0000-0000-0000-000000000002",
        tableNumber: table.trim() || undefined,
        imageBase64: capture.base64,
      });
      clearPendingCapture();
      setStatus("STUB success — QR / SMS would display here");
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Upload failed");
    } finally {
      setSending(false);
    }
  }

  return (
    <View style={styles.wrap}>
      <Text style={styles.label}>Table number</Text>
      <TextInput
        style={styles.input}
        value={table}
        onChangeText={setTable}
        placeholder="12"
        placeholderTextColor="#6B7A68"
      />
      <Pressable style={styles.btn} onPress={send} disabled={sending}>
        <Text style={styles.btnText}>{sending ? "Sending…" : "Generate guest QR"}</Text>
      </Pressable>
      <Text style={styles.status}>{status}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { flex: 1, padding: 24, gap: 12 },
  label: { color: "#C8F5C0" },
  input: {
    borderWidth: 1,
    borderColor: "#2A3B28",
    color: "#fff",
    padding: 12,
    borderRadius: 8,
  },
  btn: { backgroundColor: "#C8F5C0", padding: 14, borderRadius: 10 },
  btnText: { textAlign: "center", fontWeight: "700", color: "#0B0F0C" },
  status: { color: "#D7E3D4" },
});
