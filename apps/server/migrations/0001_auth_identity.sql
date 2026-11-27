CREATE TABLE auth_accounts (
    id TEXT PRIMARY KEY NOT NULL CHECK (length(id) > 0)
) STRICT;

CREATE TABLE auth_identities (
    provider TEXT NOT NULL CHECK (provider = 'github'),
    subject TEXT NOT NULL CHECK (
        subject GLOB '[1-9]*'
        AND subject NOT GLOB '*[^0-9]*'
        AND instr(subject, char(0)) = 0
    ),
    account_id TEXT NOT NULL REFERENCES auth_accounts (id),
    PRIMARY KEY (provider, subject),
    UNIQUE (provider, account_id)
) STRICT;
