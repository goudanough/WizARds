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
}

pub enum EffectName {
    BombExplosion = 0,
    BombSparkle = 1,
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

    asset_handles.effects.insert(
        EffectName::BombExplosion as usize,
        asset_server.add::<EffectAsset>(setup_bomb_explosion()),
    );

    asset_handles.effects.insert(
        EffectName::BombSparkle as usize,
        asset_server.add::<EffectAsset>(setup_bomb_sparkle_explosion()),
    );


    commands.insert_resource(asset_handles);
}

// Placeholder for bomb explosion, basically firework from Hanabi examples
// ToDo: create actual explosion effect
fn setup_bomb_explosion() -> EffectAsset {
    let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    color_gradient1.add_key(0.5, Vec4::new(4.0, 0.0, 0.0, 1.0));
    color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(0.0, Vec2::splat(0.1));
    size_gradient1.add_key(0.3, Vec2::splat(0.1));
    size_gradient1.add_key(1.0, Vec2::splat(0.0));

    let writer = ExprWriter::new();

    // Give a bit of variation by randomizing the age per particle. This will
    // control the starting color and starting size of particles.
    let age = writer.lit(0.).uniform(writer.lit(0.2)).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Give a bit of variation by randomizing the lifetime per particle
    let lifetime = writer.lit(0.8).uniform(writer.lit(1.2)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Add constant downward acceleration to simulate gravity
    let accel = writer.lit(Vec3::Y * -8.).expr();
    let update_accel = AccelModifier::new(accel);

    // Add drag to make particles slow down a bit after the initial explosion
    let drag = writer.lit(5.).expr();
    let update_drag = LinearDragModifier::new(drag);

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.2).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Give a bit of variation by randomizing the initial speed
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: (writer.rand(ScalarType::Float) * writer.lit(0.2) + writer.lit(0.3)).expr(),
    };

    let init_size = SetSizeModifier {
        size: bevy_hanabi::CpuValue::Single(Vec2 { x: 0.01, y: 0.01 }),
        screen_space_size: false,
    };

    EffectAsset::new(
        32768,
        Spawner::new(100.0.into(), 2.0.into(), 1000.0.into()),
        writer.finish(),
    )
    .with_name("firework")
    .init(init_pos)
    .init(init_vel)
    .init(init_age)
    .init(init_lifetime)
    .render(init_size)
    .update(update_drag)
    .update(update_accel)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient1,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient1,
        screen_space_size: false,
    })
}

fn setup_bomb_sparkle_explosion() -> EffectAsset {
    let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    color_gradient1.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
    color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(1.0, Vec2::splat(0.1));
    size_gradient1.add_key(0.3, Vec2::splat(0.1));
    size_gradient1.add_key(0.0, Vec2::splat(0.));

    let writer = ExprWriter::new();

    // Give a bit of variation by randomizing the age per particle. This will
    // control the starting color and starting size of particles.
    let age = writer.lit(0.).uniform(writer.lit(0.5)).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Give a bit of variation by randomizing the lifetime per particle
    let lifetime = writer.lit(0.5).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Add drag to make particles slow down a bit after the initial explosion
    let drag = writer.lit(5.).expr();
    let update_drag = LinearDragModifier::new(drag);

    let init_pos = SetPositionSphereModifier {
    center: writer.lit(Vec3::ZERO).expr(),
    radius: writer.lit(0.).uniform(writer.lit(0.05)).expr(),
    dimension: ShapeDimension::Surface,
    };

    let init_size = SetSizeModifier {
        size: bevy_hanabi::CpuValue::Single(Vec2 { x: 0.1, y: 0.1}),
        screen_space_size: false,
    };

    EffectAsset::new(
        32768,
        Spawner::new(2000.0.into(), 0.5.into(), 0.7.into()),
        writer.finish(),
    )
    .with_name("sparkle")
    .init(init_pos)
    .init(init_age)
    .init(init_lifetime)
    .render(init_size)
    .render(OrientModifier { mode: OrientMode::ParallelCameraDepthPlane,..Default::default() })
    .update(update_drag)
    .render(ColorOverLifetimeModifier {
        gradient: color_gradient1,
    })
    .render(SizeOverLifetimeModifier {
        gradient: size_gradient1,
        screen_space_size: false,
    })
}




