use bevy::{math::primitives, prelude::*};
use bevy_hanabi::prelude::*;
pub struct AssetHandlesPlugin;

pub enum MeshName {
    Sphere = 0,
}

pub enum MatName {
    Red = 0,
    Blue,
    Purple,
    Green,
}

pub enum EffectName {
    BombExplosion = 0,

    BombFlame = 1,
    //ParryHandEffect,
    //BombHandEffect,
}

#[derive(Resource, Default)]
pub struct AssetHandles {
    pub meshes: Vec<Handle<Mesh>>,
    pub mats: Vec<Handle<StandardMaterial>>,
    pub effects: Vec<Handle<EffectAsset>>,
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
            primitives::Sphere {
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
    asset_handles.mats.insert(
        MatName::Green as usize,
        asset_server.add::<StandardMaterial>(Color::GREEN.into()),
    );

    asset_handles.effects.insert(
        EffectName::BombExplosion as usize,
        asset_server.add::<EffectAsset>(setup_bomb_explosion()),
    );

    asset_handles.effects.insert(
        EffectName::BombFlame as usize,
        asset_server.add::<EffectAsset>(setup_bomb_flame()),
    );

    // asset_handles.effects.insert(
    //     EffectName::ParryHandEffect as usize,
    //     asset_server.add::<EffectAsset>(setup_parry_hand_effect()),
    // );

    // asset_handles.effects.insert(
    //     EffectName::BombHandEffect as usize,
    //     asset_server.add::<EffectAsset>(setup_bomb_hand_effect()),
    // );

    commands.insert_resource(asset_handles);
}

// Placeholder for bomb explosion, basically firework from Hanabi examples
// ToDo: create actual explosion effect
fn setup_bomb_explosion() -> EffectAsset {
    let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    color_gradient1.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
    color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(1.0, Vec2::splat(0.1));
    size_gradient1.add_key(0.3, Vec2::splat(0.1));
    size_gradient1.add_key(0.0, Vec2::splat(0.));

    let writer1 = ExprWriter::new();

    let age1 = writer1.lit(0.).expr();
    let init_age1 = SetAttributeModifier::new(Attribute::AGE, age1);

    let lifetime1 = writer1.lit(5.).expr();
    let init_lifetime1 = SetAttributeModifier::new(Attribute::LIFETIME, lifetime1);

    // Add constant downward acceleration to simulate gravity
    let accel1 = writer1.lit(Vec3::Y * -3.).expr();
    let update_accel1 = AccelModifier::new(accel1);

    let init_pos1 = SetPositionSphereModifier {
        center: writer1.lit(Vec3::ZERO).expr(),
        radius: writer1.lit(0.5).expr(),
        dimension: ShapeDimension::Volume,
    };

    let init_vel1 = SetVelocitySphereModifier {
        center: writer1.lit(Vec3::ZERO).expr(),
        speed: writer1.lit(1.).expr(),
    };
    EffectAsset::new(
        32768,
        // Spawner::once(6000.0.into(),true),
        Spawner::new(6000.0.into(), 6.0.into(), 5.0.into()),
        writer1.finish(),
    )
    .with_name("bomb_explosion")
    .init(init_pos1)
    // Make spawned particles move away from the emitter origin
    .init(init_vel1)
    .init(init_age1)
    .init(init_lifetime1)
    .update(update_accel1)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient1,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient1,
        screen_space_size: false,
    })
    .render(OrientModifier {
        mode: OrientMode::ParallelCameraDepthPlane,
        ..Default::default()
    })
}

//fn setup_parry_hand_effect() -> EffectAsset {}

//fn setup_bomb_hand_effect() -> EffectAsset {}

fn setup_bomb_flame() -> EffectAsset {
    let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    color_gradient1.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
    color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(1.0, Vec2::splat(0.1));
    size_gradient1.add_key(0.3, Vec2::splat(0.1));
    size_gradient1.add_key(0.0, Vec2::splat(0.));

    let writer1 = ExprWriter::new();

    let age1 = writer1.lit(0.).expr();
    let init_age1 = SetAttributeModifier::new(Attribute::AGE, age1);

    let lifetime1 = writer1.lit(0.).uniform(writer1.lit(0.3)).expr();
    let init_lifetime1 = SetAttributeModifier::new(Attribute::LIFETIME, lifetime1);

    let init_pos1 = SetPositionCone3dModifier {
        base_radius: writer1.lit(0.01).expr(),
        top_radius: writer1.lit(0.0001).expr(),
        height: writer1.lit(0.03).expr(),
        dimension: ShapeDimension::Volume,
    };

    let init_vel1 = SetVelocitySphereModifier {
        center: writer1.lit(Vec3::ZERO).expr(),
        speed: writer1.lit(1.).expr(),
    };

    let init_size = SetSizeModifier {
        size: bevy_hanabi::CpuValue::Single(Vec2 { x: 1.1, y: 1.1 }),
        screen_space_size: false,
    };

    EffectAsset::new(
        32768,
        Spawner::new(3000.0.into(), 4.0.into(), 2.0.into()),
        writer1.finish(),
    )
    .with_name("emit:rate")
    .init(init_pos1)
    // Make spawned particles move away from the emitter origin
    .init(init_vel1)
    .init(init_age1)
    .init(init_lifetime1)
    .render(init_size)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient1,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient1,
        screen_space_size: false,
    })
    .render(OrientModifier {
        mode: OrientMode::ParallelCameraDepthPlane,
        ..Default::default()
    })
}
