#[macro_export]
macro_rules! pub_struct {
    ($($derive:path),*; $name:ident {$($field:ident: $t:ty),* $(,)?}) => {
        #[derive($($derive),*)]
        pub struct $name {
            $(pub $field: $t),*
        }
    }
}

#[macro_export]
macro_rules! try_start_session {
    ($state:expr) => {
        match $state.db.client().start_session().await {
            Ok(session) => session,
            Err(_) => {
                return (
                    $crate::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json($crate::server::responses::ApiResponse::new(
                        $crate::server::responses::Status::InternalServerError,
                        "Database error: unable to start session".to_string(),
                    )),
                );
            }
        }
    };
}
