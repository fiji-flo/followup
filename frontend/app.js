// Drives the WebAuthn ceremony + signup form. No framework, plain DOM.
(function () {
  "use strict";

  const card = document.getElementById("card");
  const emailInput = document.getElementById("email");
  const continueBtn = document.getElementById("continue-btn");
  const form = document.getElementById("signup-form");
  const statusEl = document.getElementById("status");

  const t = (k) => window.i18n.t(k);

  // ---- base64url <-> ArrayBuffer helpers (WebAuthn uses URL-safe, no padding) ----
  function b64urlToBuf(value) {
    const pad = "=".repeat((4 - (value.length % 4)) % 4);
    const b64 = (value + pad).replace(/-/g, "+").replace(/_/g, "/");
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return bytes.buffer;
  }
  function bufToB64url(buf) {
    const bytes = new Uint8Array(buf);
    let bin = "";
    for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
    return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  }

  // ---- UI helpers ----
  function showStep(name) {
    card.querySelectorAll(".step").forEach((el) => {
      el.classList.toggle("hidden", el.getAttribute("data-step") !== name);
    });
  }
  function setStatus(msg, kind) {
    statusEl.textContent = msg || "";
    statusEl.className = "status" + (kind ? " " + kind : "");
  }
  function busy(on) {
    continueBtn.disabled = on;
    const submit = form.querySelector('button[type="submit"]');
    if (submit) submit.disabled = on;
  }

  // ---- fetch helper: throws an Error carrying {status, action} on non-2xx ----
  async function postJSON(url, data) {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "same-origin",
      body: JSON.stringify(data),
    });
    let body = null;
    try { body = await res.json(); } catch (_) { /* empty body */ }
    if (!res.ok) {
      const err = new Error((body && body.error) || res.statusText);
      err.status = res.status;
      err.action = body && body.action;
      throw err;
    }
    return body;
  }

  // ---- WebAuthn ceremonies ----
  async function register(email) {
    const options = await postJSON("/api/register/start", { email });
    const pk = options.publicKey;
    pk.challenge = b64urlToBuf(pk.challenge);
    pk.user.id = b64urlToBuf(pk.user.id);
    if (Array.isArray(pk.excludeCredentials)) {
      pk.excludeCredentials = pk.excludeCredentials.map((c) => ({ ...c, id: b64urlToBuf(c.id) }));
    }
    setStatus(t("msg_touch_register"), "working");
    const cred = await navigator.credentials.create({ publicKey: pk });
    await postJSON("/api/register/finish", {
      id: cred.id,
      rawId: bufToB64url(cred.rawId),
      type: cred.type,
      response: {
        attestationObject: bufToB64url(cred.response.attestationObject),
        clientDataJSON: bufToB64url(cred.response.clientDataJSON),
      },
    });
  }

  async function login(email) {
    const options = await postJSON("/api/login/start", { email });
    const pk = options.publicKey;
    pk.challenge = b64urlToBuf(pk.challenge);
    if (Array.isArray(pk.allowCredentials)) {
      pk.allowCredentials = pk.allowCredentials.map((c) => ({ ...c, id: b64urlToBuf(c.id) }));
    }
    setStatus(t("msg_touch_login"), "working");
    const cred = await navigator.credentials.get({ publicKey: pk });
    await postJSON("/api/login/finish", {
      id: cred.id,
      rawId: bufToB64url(cred.rawId),
      type: cred.type,
      response: {
        authenticatorData: bufToB64url(cred.response.authenticatorData),
        clientDataJSON: bufToB64url(cred.response.clientDataJSON),
        signature: bufToB64url(cred.response.signature),
        userHandle: cred.response.userHandle ? bufToB64url(cred.response.userHandle) : null,
      },
    });
  }

  // Register brand-new emails; fall back to authentication for known ones.
  async function verify(email) {
    try {
      await register(email);
    } catch (e) {
      if (e.status === 409 && e.action === "login") {
        await login(email);
      } else {
        throw e;
      }
    }
  }

  // ---- Handlers ----
  async function onContinue() {
    const email = (emailInput.value || "").trim();
    if (email.length < 3 || !email.includes("@")) {
      setStatus(t("msg_email_required"), "error");
      emailInput.focus();
      return;
    }
    if (!window.PublicKeyCredential || !navigator.credentials) {
      setStatus(t("msg_unsupported"), "error");
      return;
    }
    busy(true);
    setStatus("");
    try {
      await verify(email);
      setStatus("");
      showStep("form");
      const first = form.querySelector("input[type=text]");
      if (first) first.focus();
    } catch (e) {
      if (e && (e.name === "NotAllowedError" || e.name === "AbortError")) {
        setStatus(t("msg_cancelled"), "error");
      } else if (e && typeof e.status === "number") {
        setStatus(e.message || t("msg_key_failed"), "error");
      } else {
        setStatus(t("msg_key_failed"), "error");
      }
    } finally {
      busy(false);
    }
  }

  async function onSubmit(ev) {
    ev.preventDefault();
    const data = Object.fromEntries(new FormData(form).entries());
    const payload = {
      full_name: (data.full_name || "").trim(),
      company: (data.company || "").trim(),
      street: (data.street || "").trim(),
      postal_code: (data.postal_code || "").trim(),
      city: (data.city || "").trim(),
      country: (data.country || "").trim(),
      gdpr_consent: form.querySelector("#consent").checked,
    };
    if (!payload.gdpr_consent) {
      setStatus(t("msg_consent_required"), "error");
      return;
    }
    if (Object.values(payload).some((v) => v === "")) {
      setStatus(t("msg_fields_required"), "error");
      return;
    }
    busy(true);
    setStatus(t("msg_submitting"), "working");
    try {
      await postJSON("/api/signup", payload);
      setStatus("");
      showStep("success");
    } catch (e) {
      setStatus((e && e.message) || t("msg_submit_failed"), "error");
    } finally {
      busy(false);
    }
  }

  // ---- Wire up ----
  window.i18n.init();
  continueBtn.addEventListener("click", onContinue);
  emailInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); onContinue(); }
  });
  form.addEventListener("submit", onSubmit);
})();
