use std::f32::consts::TAU;

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_map_scatter::prelude::*;

const SHIP_SPEED: f32 = 2000.0;
const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, -100.0, 20.0);
const CAMERA_LOOK_AHEAD: f32 = 220.0;
const FOG_VISIBILITY: f32 = 450.0;
const STREAM_FOCUS_Y: f32 = 900.0;

#[derive(Component)]
struct Ship;

#[derive(Resource)]
struct SpaceVisuals {
    star_mesh: Handle<Mesh>,
    asteroid_mesh: Handle<Mesh>,
    debris_mesh: Handle<Mesh>,
    comet_mesh: Handle<Mesh>,
    star_small_material: Handle<StandardMaterial>,
    star_big_material: Handle<StandardMaterial>,
    asteroid_materials: Vec<Handle<StandardMaterial>>,
    debris_materials: Vec<Handle<StandardMaterial>>,
    comet_material: Handle<StandardMaterial>,
    star_small_size: Vec2,
    star_big_size: Vec2,
    star_streak_length: Vec2,
    asteroid_size: Vec2,
    debris_size: Vec2,
    comet_size: Vec2,
    comet_streak: Vec2,
    depth_range: f32,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<hud::ScatterLog>()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapScatterPlugin)
        .add_plugins(MapScatterStreamingPlugin)
        .add_systems(Startup, setup)
        .add_observer(attach_space_visuals)
        .add_systems(
            Update,
            (
                move_ship,
                follow_camera,
                hud::update_space_counter,
                hud::update_scatter_log,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Bloom::NATURAL,
        DistanceFog {
            color: Color::srgba(0.0, 0.0, 0.0, 1.0),
            falloff: FogFalloff::from_visibility_squared(FOG_VISIBILITY),
            ..Default::default()
        },
        Transform::from_translation(CAMERA_OFFSET)
            .looking_at(Vec3::new(0.0, CAMERA_LOOK_AHEAD, 0.0), Vec3::Z),
    ));
    hud::spawn_ui(&mut commands);

    let star_mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
    let asteroid_mesh = meshes.add(Mesh::from(Sphere::new(0.5)));
    let debris_mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 0.3, 0.6)));
    let comet_mesh = meshes.add(Mesh::from(Capsule3d::new(0.2, 2.0)));
    let ship_mesh = meshes.add(Mesh::from(Cuboid::new(8.0, 22.0, 6.0)));

    let small_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(10.0, 12.0, 16.0),
        ..Default::default()
    });
    let big_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(20.0, 16.0, 10.0),
        ..Default::default()
    });
    let ship_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.1, 0.16),
        emissive: LinearRgba::rgb(1.0, 3.0, 6.0),
        ..Default::default()
    });
    let asteroid_materials = vec![
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.22),
            emissive: LinearRgba::rgb(0.9, 0.85, 0.8),
            perceptual_roughness: 0.95,
            metallic: 0.05,
            ..Default::default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.22, 0.18),
            emissive: LinearRgba::rgb(0.7, 0.6, 0.5),
            perceptual_roughness: 0.9,
            metallic: 0.02,
            ..Default::default()
        }),
    ];
    let debris_materials = vec![
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.2, 0.3),
            emissive: LinearRgba::rgb(1.2, 1.5, 2.0),
            perceptual_roughness: 0.4,
            metallic: 0.6,
            ..Default::default()
        }),
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.28, 0.35),
            emissive: LinearRgba::rgb(0.6, 0.8, 1.1),
            perceptual_roughness: 0.35,
            metallic: 0.7,
            ..Default::default()
        }),
    ];
    let comet_material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(12.0, 14.0, 18.0),
        ..Default::default()
    });

    let ship_mesh_handle = ship_mesh.clone();
    let ship_material_handle = ship_material.clone();

    commands.insert_resource(SpaceVisuals {
        star_mesh,
        asteroid_mesh,
        debris_mesh,
        comet_mesh,
        star_small_material: small_material,
        star_big_material: big_material,
        asteroid_materials,
        debris_materials,
        comet_material,
        star_small_size: Vec2::new(0.4, 1.0),
        star_big_size: Vec2::new(0.9, 1.8),
        star_streak_length: Vec2::new(3.0, 8.0),
        asteroid_size: Vec2::new(1.2, 3.6),
        debris_size: Vec2::new(0.5, 1.4),
        comet_size: Vec2::new(0.8, 1.4),
        comet_streak: Vec2::new(4.0, 10.0),
        depth_range: 140.0,
    });

    let plan = asset_server.load("streaming.scatter");
    let chunk_size = Vec2::new(200.0, 200.0);
    let view_radius = IVec2::new(1, 6);

    commands.spawn((
        Ship,
        Mesh3d(ship_mesh_handle),
        MeshMaterial3d(ship_material_handle),
        Transform::from_translation(Vec3::ZERO),
        ScatterStreamSettings::new(plan, chunk_size, view_radius, 7)
            .with_focus_offset(Vec2::new(0.0, STREAM_FOCUS_Y))
            .with_raster_cell_size(1.0)
            .with_grid_halo(1),
    ));
}

fn move_ship(time: Res<Time>, mut ship: Query<&mut Transform, With<Ship>>) {
    if let Ok(mut transform) = ship.single_mut() {
        transform.translation.y += SHIP_SPEED * time.delta_secs();
    }
}

fn follow_camera(
    ship: Query<&Transform, With<Ship>>,
    mut camera: Query<&mut Transform, (With<Camera>, Without<Ship>)>,
) {
    let (Ok(ship), Ok(mut camera)) = (ship.single(), camera.single_mut()) else {
        return;
    };
    camera.translation = ship.translation + CAMERA_OFFSET;
    camera.look_at(
        ship.translation + Vec3::new(0.0, CAMERA_LOOK_AHEAD, 0.0),
        Vec3::Z,
    );
}

fn attach_space_visuals(
    event: On<ScatterStreamPlaced>,
    mut commands: Commands,
    visuals: Res<SpaceVisuals>,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(mut transform) = transforms.get_mut(event.entity) else {
        return;
    };

    let world = event.placement.position;
    let depth = (hash01(hash_vec2(world, 3)) * 2.0 - 1.0) * visuals.depth_range;
    transform.translation.z = depth;

    match event.placement.kind_id.as_str() {
        "star_big" => {
            let size = lerp(
                visuals.star_big_size.x,
                visuals.star_big_size.y,
                hash01(hash_vec2(world, 1)),
            );
            let stretch = lerp(
                visuals.star_streak_length.x,
                visuals.star_streak_length.y,
                hash01(hash_vec2(world, 2)),
            );
            transform.scale = Vec3::new(size, size * stretch, size);
            commands.entity(event.entity).insert((
                Mesh3d(visuals.star_mesh.clone()),
                MeshMaterial3d(visuals.star_big_material.clone()),
            ));
        }
        "star_small" => {
            let size = lerp(
                visuals.star_small_size.x,
                visuals.star_small_size.y,
                hash01(hash_vec2(world, 1)),
            );
            transform.scale = Vec3::splat(size);
            commands.entity(event.entity).insert((
                Mesh3d(visuals.star_mesh.clone()),
                MeshMaterial3d(visuals.star_small_material.clone()),
            ));
        }
        "asteroid" => {
            let size = lerp(
                visuals.asteroid_size.x,
                visuals.asteroid_size.y,
                hash01(hash_vec2(world, 10)),
            );
            let squash = Vec3::new(
                lerp(0.6, 1.4, hash01(hash_vec2(world, 11))),
                lerp(0.6, 1.4, hash01(hash_vec2(world, 12))),
                lerp(0.6, 1.4, hash01(hash_vec2(world, 13))),
            );
            transform.scale = squash * size;
            transform.rotation = random_rotation(world, 20);
            let material = pick_material(&visuals.asteroid_materials, hash_vec2(world, 21));
            commands.entity(event.entity).insert((
                Mesh3d(visuals.asteroid_mesh.clone()),
                MeshMaterial3d(material),
            ));
        }
        "debris" => {
            let size = lerp(
                visuals.debris_size.x,
                visuals.debris_size.y,
                hash01(hash_vec2(world, 30)),
            );
            let stretch = Vec3::new(
                lerp(0.3, 1.1, hash01(hash_vec2(world, 31))),
                lerp(0.2, 0.9, hash01(hash_vec2(world, 32))),
                lerp(0.4, 1.5, hash01(hash_vec2(world, 33))),
            );
            transform.scale = stretch * size;
            transform.rotation = random_rotation(world, 40);
            let material = pick_material(&visuals.debris_materials, hash_vec2(world, 41));
            commands.entity(event.entity).insert((
                Mesh3d(visuals.debris_mesh.clone()),
                MeshMaterial3d(material),
            ));
        }
        "comet" => {
            let size = lerp(
                visuals.comet_size.x,
                visuals.comet_size.y,
                hash01(hash_vec2(world, 50)),
            );
            let tail = lerp(
                visuals.comet_streak.x,
                visuals.comet_streak.y,
                hash01(hash_vec2(world, 51)),
            );
            transform.scale = Vec3::new(size * 0.45, size * tail, size * 0.45);
            transform.rotation =
                Quat::from_rotation_z(lerp(-0.2, 0.2, hash01(hash_vec2(world, 52))));
            commands.entity(event.entity).insert((
                Mesh3d(visuals.comet_mesh.clone()),
                MeshMaterial3d(visuals.comet_material.clone()),
            ));
        }
        _ => {
            let size = lerp(
                visuals.star_small_size.x,
                visuals.star_small_size.y,
                hash01(hash_vec2(world, 1)),
            );
            transform.scale = Vec3::splat(size);
            commands.entity(event.entity).insert((
                Mesh3d(visuals.star_mesh.clone()),
                MeshMaterial3d(visuals.star_small_material.clone()),
            ));
        }
    }
}

fn random_rotation(world: Vec2, salt: u32) -> Quat {
    let yaw = hash01(hash_vec2(world, salt)) * TAU;
    let pitch = hash01(hash_vec2(world, salt + 1)) * TAU;
    let roll = hash01(hash_vec2(world, salt + 2)) * TAU;
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll)
}

fn pick_material(materials: &[Handle<StandardMaterial>], seed: u32) -> Handle<StandardMaterial> {
    if materials.is_empty() {
        return Handle::default();
    }
    let len = materials.len();
    let idx = (hash01(seed) * len as f32).floor() as usize;
    materials[idx.min(len - 1)].clone()
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn hash_vec2(v: Vec2, salt: u32) -> u32 {
    let mut h = v.x.to_bits() ^ v.y.to_bits() ^ salt;
    h = h.wrapping_mul(0x9E3779B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EBCA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2AE35);
    h ^ (h >> 16)
}

#[inline]
fn hash01(h: u32) -> f32 {
    (h as f32) / (u32::MAX as f32)
}

mod hud {
    use std::collections::VecDeque;

    use bevy::prelude::*;
    use bevy_map_scatter::prelude::*;

    const SCATTER_LOG_LINES: usize = 8;

    #[derive(Component)]
    pub(super) struct SpaceCounterText;

    #[derive(Component)]
    pub(super) struct ScatterLogText;

    #[derive(Resource)]
    pub(super) struct ScatterLog {
        entries: VecDeque<String>,
        max_entries: usize,
    }

    impl ScatterLog {
        fn new(max_entries: usize) -> Self {
            Self {
                entries: VecDeque::new(),
                max_entries,
            }
        }

        fn push(&mut self, entry: String) {
            self.entries.push_back(entry);
            while self.entries.len() > self.max_entries {
                self.entries.pop_front();
            }
        }

        fn as_text(&self) -> String {
            self.entries.iter().cloned().collect::<Vec<_>>().join("\n")
        }
    }

    impl Default for ScatterLog {
        fn default() -> Self {
            Self::new(SCATTER_LOG_LINES)
        }
    }

    pub(super) fn spawn_ui(commands: &mut Commands) {
        commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            children![
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(16.0),
                        top: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.75)),
                    children![(
                        Text::new("Scatter log: waiting..."),
                        TextFont::from_font_size(12.0),
                        TextColor(Color::srgb(0.8, 0.88, 1.0)),
                        ScatterLogText,
                    )],
                ),
                (
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(16.0),
                        top: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.75)),
                    children![(
                        Text::new("Stars: 0 | Asteroids: 0 | Debris: 0 | Comets: 0"),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgb(0.9, 0.95, 1.0)),
                        SpaceCounterText,
                    )],
                ),
            ],
        ));
    }

    pub(super) fn update_space_counter(
        placements: Query<&ScatterStreamPlacement>,
        entities: Query<Entity>,
        mut text: Query<&mut Text, With<SpaceCounterText>>,
    ) {
        let Ok(mut text) = text.single_mut() else {
            return;
        };
        let total_entities = entities.iter().len();
        let mut stars = 0;
        let mut asteroids = 0;
        let mut debris = 0;
        let mut comets = 0;

        for placement in placements.iter() {
            match placement.kind_id.as_str() {
                "star_small" | "star_big" => stars += 1,
                "asteroid" => asteroids += 1,
                "debris" => debris += 1,
                "comet" => comets += 1,
                _ => {}
            }
        }

        text.0 = format!(
            "total entities: {total_entities} | stars: {stars} | asteroids: {asteroids} | debris: {debris} | comets: {comets}"
        );
    }

    pub(super) fn update_scatter_log(
        time: Res<Time>,
        mut reader: MessageReader<ScatterMessage>,
        chunks: Query<&ScatterStreamChunk>,
        mut log: ResMut<ScatterLog>,
        mut text: Query<&mut Text, With<ScatterLogText>>,
    ) {
        let mut changed = false;
        for message in reader.read() {
            let stamp = time.elapsed_secs();
            match &message.event {
                ScatterEvent::RunStarted { layer_count, .. } => {
                    let chunk = chunk_label(message.request_entity, &chunks);
                    log.push(format!(
                        "[{stamp:6.1}s] run start {chunk}, layers={layer_count}"
                    ));
                    changed = true;
                }
                ScatterEvent::RunFinished { result } => {
                    let chunk = chunk_label(message.request_entity, &chunks);
                    log.push(format!(
                        "[{stamp:6.1}s] run done {chunk}, placements={}",
                        result.placements.len()
                    ));
                    changed = true;
                }
                _ => {}
            }
        }

        if !changed {
            return;
        }

        let Ok(mut text) = text.single_mut() else {
            return;
        };
        let contents = log.as_text();
        text.0 = if contents.is_empty() {
            "Scatter log: waiting...".to_string()
        } else {
            contents
        };
    }

    fn chunk_label(entity: Entity, chunks: &Query<&ScatterStreamChunk>) -> String {
        if let Ok(chunk) = chunks.get(entity) {
            format!("chunk ({}, {})", chunk.id.x, chunk.id.y)
        } else {
            format!("entity {:?}", entity)
        }
    }
}
