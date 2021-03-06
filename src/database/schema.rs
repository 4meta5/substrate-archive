table! {
    accounts (address) {
        address -> Bytea,
        free_balance -> Int8,
        reserved_balance -> Int8,
        account_index -> Bytea,
        nonce -> Int4,
        create_hash -> Bytea,
        created -> Int8,
        updated -> Int8,
        active -> Bool,
    }
}

table! {
    blocks (hash) {
        id -> Int4,
        parent_hash -> Bytea,
        hash -> Bytea,
        block_num -> Int8,
        state_root -> Bytea,
        extrinsics_root -> Bytea,
        time -> Nullable<Timestamptz>,
    }
}

table! {
    events (id) {
        id -> Int4,
        block_num -> Int8,
        hash -> Bytea,
        module -> Varchar,
        event -> Varchar,
        parameters -> Jsonb,
    }
}

table! {
    inherents (id) {
        id -> Int4,
        hash -> Bytea,
        block_num -> Int8,
        module -> Varchar,
        call -> Varchar,
        parameters -> Nullable<Jsonb>,
        in_index -> Int4,
        transaction_version -> Int4,
    }
}

table! {
    signed_extrinsics (id) {
        id -> Int4,
        block_num -> Int8,
        hash -> Bytea,
        module -> Varchar,
        call -> Varchar,
        parameters -> Jsonb,
        tx_index -> Int4,
        transaction_version -> Int4,
    }
}

table! {
    storage (id) {
        id -> Int4,
        block_num -> Int8,
        hash -> Bytea,
        module -> Varchar,
        function -> Varchar,
        parameters -> Jsonb,
    }
}

joinable!(accounts -> blocks (create_hash));
joinable!(events -> blocks (hash));
joinable!(inherents -> blocks (hash));
joinable!(signed_extrinsics -> blocks (hash));
joinable!(storage -> blocks (hash));

allow_tables_to_appear_in_same_query!(
    accounts,
    blocks,
    events,
    inherents,
    signed_extrinsics,
    storage,
);
