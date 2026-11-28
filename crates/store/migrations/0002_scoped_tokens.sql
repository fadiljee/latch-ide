-- Migrasi 0002: tabel bearer token ber-scope (IT-102, IDE_SECURITY.md).
-- Hanya menyimpan hash SHA-256 token (tidak pernah menyimpan nilai token mentah).

CREATE TABLE api_tokens (
  id TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  token_hash TEXT NOT NULL UNIQUE,
  scopes TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER,
  revoked_at INTEGER,
  last_used_at INTEGER
);
CREATE INDEX idx_api_tokens_hash ON api_tokens(token_hash);
