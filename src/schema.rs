// @generated automatically by Diesel CLI.

diesel::table! {
    posts (id) {
        id -> Uuid,
        caption -> Nullable<Text>,
        image_url -> Text,
        #[max_length = 64]
        image_url_type -> Varchar,
        view_count -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
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

diesel::allow_tables_to_appear_in_same_query!(posts, users,);
