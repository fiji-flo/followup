-- Two-phase rollout switch. Phase 1 (default): attendees may only register/authenticate
-- a passkey. Phase 2 (flipped by the admin endpoint): login unlocks the contact-info
-- form. Single-row table so the state persists across restarts.
CREATE TABLE IF NOT EXISTS app_phase (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    phase2_active INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO app_phase (id, phase2_active) VALUES (1, 0);
