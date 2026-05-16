// @generated automatically by Diesel CLI.

diesel::table! {
    comments (id) {
        id -> Uuid,
        post_id -> Uuid,
        user_id -> Uuid,
        #[max_length = 255]
        username -> Varchar,
        comment -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    likes (user_id, post_id) {
        user_id -> Uuid,
        post_id -> Uuid,
        created_at -> Timestamp,
    }
}

diesel::table! {
    posts (id) {
        id -> Uuid,
        user_id -> Uuid,
        caption -> Nullable<Text>,
        #[max_length = 255]
        username -> Varchar,
        sanity_asset_id -> Text,
        view_count -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    refresh_tokens (id) {
        id -> Uuid,
        user_id -> Uuid,
        token_hash -> Text,
        expires_at -> Timestamptz,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 100]
        username -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        #[max_length = 255]
        display_name -> Nullable<Varchar>,
        bio -> Nullable<Text>,
        #[max_length = 500]
        avatar_url -> Nullable<Varchar>,
        is_active -> Bool,
        is_admin -> Bool,
        email_verified_at -> Nullable<Timestamp>,
        last_login_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(comments -> posts (post_id));
diesel::joinable!(comments -> users (user_id));
diesel::joinable!(likes -> posts (post_id));
diesel::joinable!(likes -> users (user_id));
diesel::joinable!(posts -> users (user_id));
diesel::joinable!(refresh_tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(comments, likes, posts, refresh_tokens, users,);
