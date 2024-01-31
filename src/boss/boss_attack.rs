use bevy::{
    asset::{AssetServer, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        query::{With, Without},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    math::{Quat, Vec3},
    prelude::default,
    scene::{Scene, SceneBundle},
    time::{Time, Timer, TimerMode},
    transform::components::Transform,
};
use bevy_xpbd_3d::components::{Collider, ColliderDensity, LinearVelocity, RigidBody};

use crate::player::Player;

use super::Boss;

#[derive(Resource)]
pub struct Dog(Handle<Scene>, Timer);

#[derive(Component)]
pub struct DogDog(Timer);
pub fn init_dog(mut commands: Commands, asset_server: Res<AssetServer>) {
    let dog = asset_server.load("dog.glb#Scene0");
    commands.insert_resource(Dog(dog, Timer::from_seconds(5.0, TimerMode::Repeating)));
}

pub fn spawn_and_launch_dog(
    mut dog: ResMut<Dog>,
    mut commands: Commands,
    boss_query: Query<&Transform, (With<Boss>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Boss>)>,
    time: Res<Time>,
) {
    if dog.1.tick(time.delta()).just_finished() {
        let boss_transform = boss_query.single();
        let player_transform = player_query.single();
    
        let player_pos = Vec3::new(player_transform.translation.x, 0.0, player_transform.translation.z);
        let boss_pos = Vec3::new(boss_transform.translation.x, 0.0, boss_transform.translation.z);
        let launch_dir = (player_pos - boss_pos).normalize();

        let forward = boss_transform.forward();
        let left = Vec3::new(-forward.z, 0.0, forward.x);
        let dog_position = boss_transform.translation + left * 2.0;
        commands.spawn((

            SceneBundle {
                scene: dog.0.clone(),
                transform: Transform::from_translation(dog_position)
                    .with_scale(Vec3::new(0.5, 0.5, 0.5)).with_rotation(Quat::from_rotation_y(90.0f32.to_radians())),
                ..default()
            },
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5), 
            LinearVelocity(launch_dir * 20.0),
            DogDog(Timer::from_seconds(5.0, TimerMode::Once)),
            ColliderDensity(1000.0),
        ));
    }
}

pub fn dog_update(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DogDog)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            commands.entity(entity).despawn();
        }
    }
}