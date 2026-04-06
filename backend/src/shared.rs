pub mod api {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct ApiMessage {
        pub message: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct ApiErrorBody {
        pub error: String,
        pub message: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct AppInfoResponse {
        pub name: String,
        pub phase: u8,
        pub frontend: String,
        pub backend: String,
        pub version: String,
        pub web_dist_dir: String,
    }
}

pub mod auth {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct LoginRequest {
        pub username: String,
        pub password: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct CurrentUserResponse {
        pub username: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct CsrfTokenResponse {
        pub token: String,
    }
}

pub mod users {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct UserSummary {
        pub username: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct CreateUserRequest {
        pub username: String,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct LinksPayload {
        pub links: Vec<String>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct UserLinksResponse {
        pub username: String,
        pub links: Vec<String>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct UserOrderPayload {
        pub order: Vec<String>,
    }
}
