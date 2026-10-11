import { registerProvider, type OAuthProvider } from "./provider";

const AUTHORIZE_URL = "https://github.com/login/oauth/authorize";

export const githubProvider: OAuthProvider = {
  name: "github",
  authorizationUrl(state) {
    return `${AUTHORIZE_URL}?client_id=acme&state=${encodeURIComponent(state)}&scope=user:email`;
  },
  async exchangeCode(code) {
    const response = await fetch("https://github.com/login/oauth/access_token", {
      method: "POST",
      headers: { accept: "application/json" },
      body: new URLSearchParams({ code }),
    });
    const { email, verified } = (await response.json()) as { email: string; verified: boolean };
    return { email, verified };
  },
};

registerProvider(githubProvider);
