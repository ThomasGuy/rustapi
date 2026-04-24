// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Uuid,
        user_id -> Int4,
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
