use bevy::prelude::*;
pub struct AssetHandlesPlugin;

pub enum MeshName {
    Sphere = 0,
}

pub enum MatName {
    Red = 0,
    Blue,
    Purple,
}

#[derive(Resource, Default)]
pub struct AssetHandles {
    pub meshes: Vec<Handle<Mesh>>,
    pub mats: Vec<Handle<StandardMaterial>>,
}

impl Plugin for AssetHandlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut asset_handles = AssetHandles::default();
    asset_handles.meshes.insert(
        MeshName::Sphere as usize,
        asset_server.add::<Mesh>(
            shape::UVSphere {
                radius: 0.1,
                ..default()
            }
            .into(),
        ),
    );
    asset_handles.mats.insert(
        MatName::Red as usize,
        asset_server.add::<StandardMaterial>(Color::RED.into()),
    );
    asset_handles.mats.insert(
        MatName::Blue as usize,
        asset_server.add::<StandardMaterial>(Color::BLUE.into()),
    );
    asset_handles.mats.insert(
        MatName::Purple as usize,
        asset_server.add::<StandardMaterial>(Color::PURPLE.into()),
    );

    commands.insert_resource(asset_handles);
}
