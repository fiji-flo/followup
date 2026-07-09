// Bilingual (EN/DE) string tables + a tiny apply-on-[data-i18n] helper.
(function () {
  const STRINGS = {
    en: {
      brand_badge: "Enterprise",
      hero_eyebrow: "Pre-launch access",
      hero_title: "Enterprise browsing, built by Mozilla.",
      hero_lede:
        "You picked up a security key at our booth. Use it to claim your spot on the Firefox Enterprise pre-launch list — and we'll send materials and swag when we go live.",
      perk_1: "Early access to Firefox Enterprise evaluation builds",
      perk_2: "Deep-dive security & manageability briefings",
      perk_3: "Launch-day swag shipped to your door",
      key_note:
        "Your security key proves you met us in person — no password needed.",
      email_title: "Claim your spot",
      email_help:
        "Enter your email, then tap your security key to verify it's you.",
      label_email: "Work email",
      ph_email: "you@company.com",
      btn_continue: "Continue with security key",
      form_title: "You're verified — tell us where to send it",
      form_title_edit: "Update your details",
      form_intro:
        "We'll use these details to ship your swag and info material.",
      label_full_name: "Full name",
      label_company: "Company",
      label_street: "Street & number",
      label_postal_code: "Postal code",
      label_city: "City",
      label_country: "Country",
      consent_text:
        "I agree that Mozilla may store these details to send me Firefox Enterprise information and swag.",
      privacy_link: "Mozilla Privacy Notice",
      btn_submit: "Send me the good stuff",
      btn_update: "Update my details",
      btn_edit: "Edit my details",
      phase1_title: "You're verified!",
      phase1_body:
        "Stay tuned. We'll send you an email when we enter our pre-launch phase.",
      success_title: "You're on the list!",
      success_body:
        "Thanks — we'll be in touch as Firefox Enterprise gets closer to launch.",
      footer_powered: "Powered by Mozilla",
      footer_privacy: "Privacy",
      // JS-driven status messages
      msg_email_required: "Please enter a valid email address.",
      msg_unsupported:
        "This browser doesn't support security keys. Try a recent Chrome, Edge, Firefox or Safari.",
      msg_touch_register: "Touch your security key to register…",
      msg_touch_login: "Welcome back — touch your security key to sign in…",
      msg_cancelled: "That was cancelled. Tap the button to try again.",
      msg_key_failed: "We couldn't verify that security key. Please try again.",
      msg_submitting: "Saving your details…",
      msg_submit_failed:
        "Something went wrong saving your details. Please try again.",
      msg_consent_required:
        "Please tick the consent box so we can send your swag.",
      msg_fields_required: "Please fill in all fields.",
    },
    de: {
      brand_badge: "Enterprise",
      hero_eyebrow: "Vorab-Zugang",
      hero_title: "Enterprise-Browsing, entwickelt von Mozilla.",
      hero_lede:
        "Sie haben an unserem Stand einen Sicherheitsschlüssel erhalten. Sichern Sie sich damit Ihren Platz auf der Firefox-Enterprise-Vorabliste — zum Launch senden wir Ihnen Material und Swag.",
      perk_1: "Früher Zugang zu Firefox-Enterprise-Evaluierungsversionen",
      perk_2: "Ausführliche Briefings zu Sicherheit & Verwaltbarkeit",
      perk_3: "Swag zum Launch direkt zu Ihnen nach Hause",
      key_note:
        "Ihr Sicherheitsschlüssel belegt, dass wir uns persönlich getroffen haben — kein Passwort nötig.",
      email_title: "Platz sichern",
      email_help:
        "Geben Sie Ihre E-Mail ein und bestätigen Sie sich mit Ihrem Sicherheitsschlüssel.",
      label_email: "Geschäftliche E-Mail",
      ph_email: "sie@firma.de",
      btn_continue: "Mit Sicherheitsschlüssel fortfahren",
      form_title: "Verifiziert — wohin dürfen wir es schicken?",
      form_title_edit: "Ihre Angaben aktualisieren",
      form_intro:
        "Wir nutzen diese Angaben, um Ihnen Swag und Infomaterial zuzusenden.",
      label_full_name: "Vollständiger Name",
      label_company: "Unternehmen",
      label_street: "Straße & Hausnummer",
      label_postal_code: "PLZ",
      label_city: "Stadt",
      label_country: "Land",
      consent_text:
        "Ich bin einverstanden, dass Mozilla diese Angaben speichert, um mir Firefox-Enterprise-Informationen und Swag zuzusenden.",
      privacy_link: "Mozilla-Datenschutzhinweis",
      btn_submit: "Schickt mir die guten Sachen",
      btn_update: "Angaben aktualisieren",
      btn_edit: "Angaben bearbeiten",
      phase1_title: "Sie sind verifiziert!",
      phase1_body:
        "Bleiben Sie dran. Wir schicken Ihnen eine E-Mail, sobald unsere Vorab-Phase beginnt.",
      success_title: "Sie sind auf der Liste!",
      success_body:
        "Danke — wir melden uns, sobald der Firefox-Enterprise-Launch näher rückt.",
      footer_powered: "Bereitgestellt von Mozilla",
      footer_privacy: "Datenschutz",
      msg_email_required: "Bitte geben Sie eine gültige E-Mail-Adresse ein.",
      msg_unsupported:
        "Dieser Browser unterstützt keine Sicherheitsschlüssel. Bitte nutzen Sie ein aktuelles Chrome, Edge, Firefox oder Safari.",
      msg_touch_register:
        "Berühren Sie Ihren Sicherheitsschlüssel zur Registrierung…",
      msg_touch_login:
        "Willkommen zurück — berühren Sie Ihren Sicherheitsschlüssel zur Anmeldung…",
      msg_cancelled:
        "Vorgang abgebrochen. Tippen Sie erneut auf die Schaltfläche.",
      msg_key_failed:
        "Der Sicherheitsschlüssel konnte nicht verifiziert werden. Bitte erneut versuchen.",
      msg_submitting: "Ihre Angaben werden gespeichert…",
      msg_submit_failed:
        "Beim Speichern ist etwas schiefgelaufen. Bitte erneut versuchen.",
      msg_consent_required:
        "Bitte bestätigen Sie die Einwilligung, damit wir Ihren Swag senden können.",
      msg_fields_required: "Bitte füllen Sie alle Felder aus.",
    },
  };

  let currentLang = "en";

  function detect() {
    const saved = localStorage.getItem("lang");
    if (saved === "en" || saved === "de") return saved;
    return (navigator.language || "en").toLowerCase().startsWith("de")
      ? "de"
      : "en";
  }

  function t(key) {
    const table = STRINGS[currentLang] || STRINGS.en;
    return table[key] || STRINGS.en[key] || key;
  }

  function apply(lang) {
    currentLang = lang === "de" ? "de" : "en";
    localStorage.setItem("lang", currentLang);
    document.documentElement.lang = currentLang;

    document.querySelectorAll("[data-i18n]").forEach((el) => {
      el.textContent = t(el.getAttribute("data-i18n"));
    });
    document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
      el.setAttribute(
        "placeholder",
        t(el.getAttribute("data-i18n-placeholder")),
      );
    });
    document.querySelectorAll("[data-lang]").forEach((b) => {
      b.classList.toggle("active", b.getAttribute("data-lang") === currentLang);
    });
  }

  window.i18n = {
    t,
    apply,
    init() {
      apply(detect());
      document.querySelectorAll("[data-lang]").forEach((b) => {
        b.addEventListener("click", () => apply(b.getAttribute("data-lang")));
      });
    },
    get lang() {
      return currentLang;
    },
  };
})();
