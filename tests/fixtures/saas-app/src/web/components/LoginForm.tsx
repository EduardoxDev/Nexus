import { useState } from "react";

/** Formulário de login com e-mail e senha. */
export function LoginForm({ onSuccess }: { onSuccess: () => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  async function submit() {
    const res = await fetch("/api/auth/login", { method: "POST", body: JSON.stringify({ email, password }) });
    if (res.ok) onSuccess();
  }
  return (
    <form onSubmit={(e) => { e.preventDefault(); void submit(); }}>
      <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} />
      <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
      <button type="submit">Entrar</button>
    </form>
  );
}
