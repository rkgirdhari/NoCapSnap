import { useState } from "react";
import { Pressable, StyleSheet, Text, TextInput, View } from "react-native";
import { router } from "expo-router";
import { loginStub } from "@/api/auth";

export default function LoginScreen() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function onLogin() {
    try {
      await loginStub(email, password);
      router.replace("/capture");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Login failed");
    }
  }

  return (
    <View style={styles.wrap}>
      <Text style={styles.label}>Staff email</Text>
      <TextInput
        autoCapitalize="none"
        keyboardType="email-address"
        style={styles.input}
        value={email}
        onChangeText={setEmail}
        placeholder="chef@restaurant.com"
        placeholderTextColor="#6B7A68"
      />
      <Text style={styles.label}>Password</Text>
      <TextInput
        secureTextEntry
        style={styles.input}
        value={password}
        onChangeText={setPassword}
        placeholder="••••••••"
        placeholderTextColor="#6B7A68"
      />
      {error ? <Text style={styles.error}>{error}</Text> : null}
      <Pressable style={styles.btn} onPress={onLogin}>
        <Text style={styles.btnText}>Enter kitchen</Text>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { flex: 1, padding: 24, gap: 10 },
  label: { color: "#C8F5C0" },
  input: {
    borderWidth: 1,
    borderColor: "#2A3B28",
    color: "#fff",
    padding: 12,
    borderRadius: 8,
  },
  error: { color: "#FF6B6B" },
  btn: { backgroundColor: "#C8F5C0", padding: 14, borderRadius: 10, marginTop: 8 },
  btnText: { textAlign: "center", fontWeight: "700", color: "#0B0F0C" },
});
