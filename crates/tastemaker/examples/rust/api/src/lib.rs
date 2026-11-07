//! The todo API: its domain, its use cases, the adapters of its ports and its routes.

pub mod domain {
    pub mod todo {
        pub mod ports;
        pub mod titles;
        pub mod types;
    }
    pub mod user {
        pub mod types;
    }
}
pub mod inbound {
    pub mod use_cases {
        pub mod archive_todo {
            pub mod archive;
        }
        pub mod complete_todo {
            pub mod use_case;
        }
        pub mod create_todo {
            pub mod use_case;
        }
        pub mod list_todos {
            pub mod use_case;
        }
    }
}
pub mod outbound {
    pub mod todos {
        pub mod adapter;
    }
}
pub mod routes {
    pub mod todos {
        pub mod routes;
    }
}
