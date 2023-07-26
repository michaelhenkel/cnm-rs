pub mod jnx {
    pub mod jet {
        pub mod authentication {
            include!("jnx.jet.authentication.rs");
        }
        pub mod common {
            include!("jnx.jet.common.rs");
        }
        pub mod management {
            include!("jnx.jet.management.rs");
        }
    }
}
