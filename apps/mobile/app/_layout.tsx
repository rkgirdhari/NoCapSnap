import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";

export default function RootLayout() {
  return (
    <>
      <StatusBar style="light" />
      <Stack
        screenOptions={{
          headerStyle: { backgroundColor: "#0B0F0C" },
          headerTintColor: "#C8F5C0",
          contentStyle: { backgroundColor: "#0B0F0C" },
        }}
      >
        <Stack.Screen name="index" options={{ title: "CapSnap" }} />
        <Stack.Screen name="login" options={{ title: "Staff login" }} />
        <Stack.Screen name="capture" options={{ title: "Capture dish" }} />
        <Stack.Screen name="preview" options={{ title: "Preview" }} />
        <Stack.Screen name="send" options={{ title: "Send to guest" }} />
        <Stack.Screen name="history" options={{ title: "Photo log" }} />
      </Stack>
    </>
  );
}
