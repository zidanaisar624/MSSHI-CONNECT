// Secure Loopback API client for MSSHI Installer & Connect (CSPRNG X-MSSHI-Token)
let sessionToken = "";

export function initSessionToken() {
  const hash = window.location.hash || "";
  const match = hash.match(/t=([a-f0-9]{16,128})/i);
  if (match) {
    sessionToken = match[1];
    window.history.replaceState(null, "", window.location.pathname + window.location.search);
  }
  return sessionToken;
}

export async function apiRequest(path, { method = "GET", body = null } = {}) {
  const headers = { "X-MSSHI-Token": sessionToken };
  const opts = { method, headers };
  if (body !== null) {
    headers["Content-Type"] = "application/json";
    opts.body = JSON.stringify(body);
  }
  const resp = await fetch(path, opts);
  const data = await resp.json().catch(() => ({}));
  if (!resp.ok) {
    throw new Error(data.error || `HTTP ${resp.status}`);
  }
  return data;
}
