import { Link } from "expo-router";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { COMPANY } from "@nocapsnap/shared";

export default function HomeScreen() {
  return (
    <View style={styles.wrap}>
      <Text style={styles.kicker}>{COMPANY.legalName}</Text>
      <Text style={styles.title}>CapSnap</Text>
      <Text style={styles.sub}>
        Snap the real plate. Tie the review to that dish. No cap.
      </Text>
      <Text style={styles.founder}>Founder: {COMPANY.founder}</Text>

      <Link href="/login" asChild>
        <Pressable style={styles.btn}>
          <Text style={styles.btnText}>Staff login</Text>
        </Pressable>
      </Link>
      <Link href="/capture" asChild>
        <Pressable style={[styles.btn, styles.ghost]}>
          <Text style={styles.ghostText}>Open camera (stub)</Text>
        </Pressable>
      </Link>
      <Link href="/history" asChild>
        <Pressable style={[styles.btn, styles.ghost]}>
          <Text style={styles.ghostText}>Photo log</Text>
        </Pressable>
      </Link>
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { flex: 1, padding: 24, justifyContent: "center", gap: 12 },
  kicker: { color: "#7A9B76", letterSpacing: 1.2, textTransform: "uppercase" },
  title: { color: "#C8F5C0", fontSize: 42, fontWeight: "800" },
  sub: { color: "#D7E3D4", fontSize: 16, lineHeight: 22 },
  founder: { color: "#7A9B76", marginBottom: 16 },
  btn: {
    backgroundColor: "#C8F5C0",
    padding: 14,
    borderRadius: 10,
    alignItems: "center",
  },
  btnText: { color: "#0B0F0C", fontWeight: "700" },
  ghost: { backgroundColor: "transparent", borderWidth: 1, borderColor: "#C8F5C0" },
  ghostText: { color: "#C8F5C0", fontWeight: "700" },
});
