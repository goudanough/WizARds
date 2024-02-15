pub mod depth;
pub mod scene;

#[macro_export]
macro_rules! oxr {
    ($e:expr) => {{
        let result = unsafe { $e };
        if result != bevy_oxr::xr::sys::Result::SUCCESS {
            panic!(r#"{} failed with error "{}""#, stringify!($e), result)
        }
    }};
}
