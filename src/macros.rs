pub use crate::proc_macros::*;

#[macro_export]
macro_rules! project_str (
    ($path:literal) => { $crate::proc_macros::project_str!($path) };
    ($path:literal, $($args:tt)*) => {
        std::format!($crate::proc_macros::project_str!($path), $($args)*)
    };
);
