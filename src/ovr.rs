use bevy::prelude::*;
use jni::{objects::JObject, JNIEnv};
use std::ffi::CString;
use std::os::raw::c_char;

type OvrID = u64;
type OvrRequest = u64;

#[link(name = "ovrplatformloader")]
extern "C" {
    #[link_name = "ovr_GetLoggedInUserID"]
    fn get_logged_in_user_id() -> OvrID;

    #[link_name = "ovr_PlatformInitializeAndroid"]
    fn platform_initialize_android(
        app_id: *const c_char,
        activity_object: JObject,
        jni: JNIEnv,
    ) -> OvrRequest;
}

pub(super) struct OvrPlugin;

impl Plugin for OvrPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Ovr::new());
    }
}

#[derive(Resource)]
pub struct Ovr;

impl Ovr {
    fn new() -> Self {
        let android_app = bevy::winit::ANDROID_APP.get().unwrap();
        let app_name = CString::new("org.goudanough.wizARds").unwrap();
        unsafe {
            platform_initialize_android(
                app_name.as_ptr(),
                JObject::from_raw(android_app.activity_as_ptr() as *mut _),
                jni::JavaVM::from_raw(android_app.vm_as_ptr() as *mut _)
                    .unwrap()
                    .get_env()
                    .unwrap(),
            )
        };
        Self
    }

    pub fn get_logged_in_user_id(&self) -> OvrID {
        unsafe { get_logged_in_user_id() }
    }
}
