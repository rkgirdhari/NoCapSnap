import { StyleSheet, Text, View } from "react-native";

export default function HistoryScreen() {
  return (
    <View style={styles.wrap}>
      <Text style={styles.title}>Photo log</Text>
      <Text style={styles.copy}>STUB: list captured plates for this location.</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { flex: 1, padding: 24 },
  title: { color: "#C8F5C0", fontSize: 24, fontWeight: "700" },
  copy: { color: "#D7E3D4", marginTop: 8 },
});
