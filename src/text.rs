use bevy::{
    ecs::system::Command,
    prelude::*,
    render::{
        camera::RenderTarget,
        mesh::shape::Quad,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
};
use bevy_oxr::xr_input::trackers::{OpenXRLeftEye, OpenXRRightEye};

pub struct TextPlugin;
impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                setup_new_texts,
                update_texts.after(setup_new_texts),
                despawn_timed_text,
                point_text_at_player,
            ),
        );
    }
}

#[derive(Component)]
pub struct Text {
    text: String,
    ui_node: Entity,
    text_node: Entity,
}

#[derive(Component)]
pub struct TextTimer(pub Timer);

impl Text {
    pub fn new(text: String) -> Self {
        Text {
            text,
            ui_node: Entity::PLACEHOLDER,
            text_node: Entity::PLACEHOLDER,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }
}

fn setup_new_texts(
    mut q: Query<(Entity, &Transform, &mut Text), Added<Text>>,
    mut cmds: Commands,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (id, trans, mut text) in q.iter_mut() {
        let size = Extent3d {
            width: 800,
            height: 80,
            depth_or_array_layers: 1,
        };
        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                dimension: TextureDimension::D2,
                mip_level_count: 1,
                sample_count: 1,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        // zero it out
        image.resize(size);
        let image_handle = images.add(image);
        let mut quad = Quad::new(Vec2::new(5.0, 0.5));
        quad.flip = true;
        let rect_handle = meshes.add(quad);
        let mat_handle = mats.add(StandardMaterial {
            base_color_texture: Some(image_handle.clone()),
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            unlit: true,
            ..default()
        });
        cmds.entity(id).insert(PbrBundle {
            mesh: rect_handle,
            material: mat_handle,
            transform: trans.clone(), // don't overwrite existing transform
            ..default()
        });

        let camera = cmds
            .spawn(Camera2dBundle {
                camera: Camera {
                    order: -1,
                    target: RenderTarget::Image(image_handle.clone()),
                    ..default()
                },
                ..default()
            })
            .id();
        let mut text_node = Entity::PLACEHOLDER;
        let ui_node = cmds
            .spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::rgba_u8(0, 0, 0, 0).into(),
                    ..default()
                },
                TargetCamera(camera),
            ))
            .with_children(|parent| {
                text_node = parent
                    .spawn(TextBundle::from_section(
                        &text.text,
                        TextStyle {
                            font_size: 10.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ))
                    .id();
            })
            .id();
        text.ui_node = ui_node;
        text.text_node = text_node;
    }
}

fn update_texts(ts: Query<&Text, Changed<Text>>, mut text_nodes: Query<&mut bevy::text::Text>) {
    for t in ts.iter() {
        if t.text_node == Entity::PLACEHOLDER {
            continue;
        }
        text_nodes.get_mut(t.text_node).unwrap().sections[0].value = t.text.clone();
    }
}

fn despawn_timed_text(
    mut commands: Commands,
    mut ts: Query<(&mut TextTimer, Entity)>,
    time: Res<Time>,
) {
    for (mut t, e) in ts.iter_mut() {
        if t.0.tick(time.delta()).just_finished() {
            commands.add(RemoveText(e));
            commands.entity(e).despawn();
        }
    }
}

fn point_text_at_player(
    mut ts: Query<
        (&mut Transform, &GlobalTransform),
        (With<Text>, Without<OpenXRLeftEye>, Without<OpenXRRightEye>),
    >,
    left_eye: Query<&Transform, With<OpenXRLeftEye>>,
    right_eye: Query<&Transform, With<OpenXRRightEye>>,
) {
    let head_pos = (left_eye.single().translation + right_eye.single().translation) / 2.0;
    for (mut t, gt) in ts.iter_mut() {
        let gt = gt.compute_transform();
        let target_gt = gt.looking_at(head_pos, Vec3::Y);
        let rot = target_gt.rotation - gt.rotation;
        t.rotation = t.rotation + rot;
    }
}

pub struct RemoveText(pub Entity);
impl Command for RemoveText {
    fn apply(self, w: &mut World) {
        let mut q = w.query::<&mut Text>();
        let ui_node = q
            .get(w, self.0)
            .expect("tried to remove text from an entity without text!")
            .ui_node;
        w.entity_mut(ui_node).despawn();

        // can't hold &w across the previous line so we have to query t again
        let mut t = q.get_mut(w, self.0).unwrap();
        t.ui_node = Entity::PLACEHOLDER;
        t.text_node = Entity::PLACEHOLDER;
        w.entity_mut(self.0).remove::<PbrBundle>();
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        if self.ui_node != Entity::PLACEHOLDER {
            warn!(
                "Text component was dropped without using the RemoveText command! \
                This leaks memory!"
            );
        }
    }
}
