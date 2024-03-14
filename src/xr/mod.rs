use bevy::ecs::{schedule::States, system::Resource};
use bevy_oxr::xr::sys;

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

// Set the
#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum SceneState {
    #[default]
    Uninit, // Default state, do nothing
    Scanning,      // Set the state to this to force the device to scan
    QueryingScene, // Waits for a SpaceQueryResultsAvailableFB event and uses this to populate the scene
    Locating,
    EnableStoreShare,
    Uploading,
    Done, // Finished initialization
}

// This resource represents the anchor that the game will center around
// This struct is to retain the XrSpace handle representing the mesh of the room
#[derive(Resource, Default)]
pub struct SpatialAnchors {
    pub mesh: sys::Space,
    pub floor: sys::Space,
    pub walls: Vec<sys::Space>,
    pub ceiling: sys::Space,
    pub position: sys::Space,
}
