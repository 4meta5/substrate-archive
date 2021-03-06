CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    block_num bigint check (block_num >= 0 and block_num < '9223372036854775807'::bigint) NOT NULL,
    hash bytea NOT NULL REFERENCES blocks(hash) ON DELETE CASCADE ON UPDATE CASCADE,
    module varchar NOT NULL,
    event varchar NOT NULL,
    parameters jsonb NOT NULL
);
