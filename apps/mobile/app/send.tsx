import { useState } from "react";
import {
  Image,
  Pressable,
  ScrollView,
  Share,
  StyleSheet,
  Text,
  TextInput,
  View,
} from "react-native";
import { router } from "expo-router";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { photoVerificationCode, type Photo } from "@nocapsnap/shared";
import { capturePhoto } from "@/api/photos";
import { clearPendingCapture, getPendingCapture } from "@/state/pendingCapture";

export default function SendScreen() {
  const [table, setTable] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [photo, setPhoto] = useState<Photo | null>(null);

  async function send() {
    const capture = getPendingCapture();
    if (!capture) {
      setStatus("No photo to send — snap the plate first.");
      return;
    }
    setSending(true);
    setStatus("Uploading…");
    try {
      const res = await capturePhoto({
        locationId: "00000000-0000-0000-0000-000000000001",
        menuItemId: "00000000-0000-0000-0000-000000000002",
        tableNumber: table.trim() || undefined,
        imageBase64: capture.base64,
      });
      clearPendingCapture();
      setStatus(null);
      setPhoto(res.photo);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Upload failed");
    } finally {
      setSending(false);
    }
  }

  if (photo) return <GuestQr photo={photo} />;

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
      {status ? <Text style={styles.status}>{status}</Text> : null}
    </View>
  );
}

function GuestQr({ photo }: { photo: Photo }) {
  const [qrFailed, setQrFailed] = useState(false);
  const insets = useSafeAreaInsets();
  const code = photoVerificationCode(photo.id);

  async function shareLink() {
    try {
      await Share.share({
        message: `Here's the photo of your dish — tap to leave a review: ${photo.reviewUrl}`,
      });
    } catch {
      // Share sheet dismissed or unavailable; the link is still on screen.
    }
  }

  return (
    <ScrollView
      contentContainerStyle={[styles.qrWrap, { paddingBottom: 24 + insets.bottom }]}
    >
      <Text style={styles.qrTitle}>Show this to the guest</Text>
      <Text style={styles.qrSub}>They scan it to see this plate and leave a review.</Text>

      <View style={styles.qrCard}>
        {qrFailed ? (
          <Text style={styles.qrError}>
            Couldn't load the QR code. Share the review link instead.
          </Text>
        ) : (
          <Image
            source={{ uri: photo.qrCodeUrl }}
            style={styles.qr}
            onError={() => setQrFailed(true)}
            accessibilityLabel="Review QR code"
          />
        )}
      </View>

      <Text style={styles.meta}>
        #{code}
        {photo.tableNumber ? ` · Table ${photo.tableNumber}` : ""}
      </Text>
      <Text style={styles.link} selectable>
        {photo.reviewUrl}
      </Text>

      <Pressable style={styles.btn} onPress={shareLink}>
        <Text style={styles.btnText}>Share review link</Text>
      </Pressable>
      <Pressable style={[styles.btn, styles.ghost]} onPress={() => router.dismissTo("/capture")}>
        <Text style={styles.ghostText}>Snap next plate</Text>
      </Pressable>
    </ScrollView>
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
  btn: { backgroundColor: "#C8F5C0", padding: 14, borderRadius: 10, alignSelf: "stretch" },
  btnText: { textAlign: "center", fontWeight: "700", color: "#0B0F0C" },
  ghost: { backgroundColor: "transparent", borderWidth: 1, borderColor: "#C8F5C0" },
  ghostText: { textAlign: "center", fontWeight: "700", color: "#C8F5C0" },
  status: { color: "#D7E3D4" },
  qrWrap: { padding: 24, gap: 12, alignItems: "center" },
  qrTitle: { color: "#C8F5C0", fontSize: 22, fontWeight: "800" },
  qrSub: { color: "#D7E3D4", textAlign: "center" },
  // White card keeps the QR's quiet zone high-contrast against the dark UI.
  qrCard: {
    backgroundColor: "#FFFFFF",
    borderRadius: 16,
    padding: 12,
    width: 280,
    height: 280,
    alignItems: "center",
    justifyContent: "center",
    marginVertical: 8,
  },
  qr: { width: 256, height: 256 },
  qrError: { color: "#0B0F0C", textAlign: "center", padding: 16 },
  meta: { color: "#C8F5C0", fontSize: 18, fontWeight: "700", letterSpacing: 1 },
  link: { color: "#7A9B76", textAlign: "center", marginBottom: 8 },
});
