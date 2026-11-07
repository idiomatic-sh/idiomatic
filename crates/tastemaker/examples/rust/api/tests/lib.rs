//! The tests of the todo API: one module for each source file, at the path that mirrors it under `src/`.

mod domain {
    mod todo {
        mod ports;
        mod titles;
        mod types;
    }
}
mod inbound {
    mod use_cases {
        mod archive_todo {
            mod archive;
        }
        mod complete_todo {
            mod use_case;
        }
        mod create_todo {
            mod use_case;
        }
        mod list_todos {
            mod use_case;
        }
    }
}
mod outbound {
    mod todos {
        mod adapter;
    }
}
mod routes {
    mod todos {
        mod routes;
    }
}
