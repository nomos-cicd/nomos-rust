use chrono::{DateTime, Utc};

struct TextCredentialParameter {
    pub username: String,
    pub password: String,
}

struct SshCredentialParameter {
    pub username: String,
    pub private_key: String,
}

enum CredentialType {
    Text(TextCredentialParameter),
    Ssh(SshCredentialParameter),
}

struct Credential {
    pub id: String,
    pub value: CredentialType,
    pub readonly: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
