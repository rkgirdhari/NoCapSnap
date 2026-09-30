import { Image, Pressable, StyleSheet, Text, View } from "react-native";
import { router } from "expo-router";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { getPendingCapture } from "@/state/pendingCapture";

export default function PreviewScreen() {
  const capture = getPendingCapture();
  const insets = useSafeAreaInsets();

  if (!capture) {
    return (
      <View style={styles.wrap}>
        <Text style={styles.stamp}>No photo yet — go back and snap the plate.</Text>
        <Pressable style={styles.btn} onPress={() => router.replace("/capture")}>
          <Text style={styles.btnText}>Open camera</Text>
        </Pressable>
      </View>
    );
  }

  return (
    <View style={[styles.wrap, { paddingBottom: 16 + insets.bottom }]}>
      <Image source={{ uri: capture.uri }} style={styles.image} />
      <Text style={styles.stamp}>STUB watermark: CapSnap · timestamp · dish</Text>
      <Pressable style={styles.btn} onPress={() => router.push("/send")}>
        <Text style={styles.btnText}>Use this plate</Text>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { flex: 1, padding: 16, gap: 12 },
  image: { flex: 1, borderRadius: 12, backgroundColor: "#222" },
  stamp: { color: "#C8F5C0", textAlign: "center" },
  btn: { backgroundColor: "#C8F5C0", padding: 14, borderRadius: 10 },
  btnText: { textAlign: "center", fontWeight: "700", color: "#0B0F0C" },
});
