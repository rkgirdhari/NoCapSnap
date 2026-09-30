export async function loginStub(email: string, _password: string) {
  if (!email.includes("@")) {
    throw new Error("Use a staff email");
  }
  // STUB: POST /api/auth/login → store JWT in expo-secure-store
  return { token: "stub-token", staffId: "stub-staff" };
}
