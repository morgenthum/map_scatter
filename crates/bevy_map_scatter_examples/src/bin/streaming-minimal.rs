use bevy::prelude::*;
use bevy_map_scatter::prelude::*;

const CHUNK_SIZE: f32 = 12.0;
const VIEW_RADIUS: i32 = 1;
const DRIFT_SPEED: f32 = 6.0;
const SWAY_AMPLITUDE: f32 = 8.0;
const SWAY_SPEED: f32 = 0.6;

#[derive(Component)]
struct Anchor;

#[derive(Resource)]
struct Visuals {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapScatterPlugin)
        .add_plugins(MapScatterStreamingPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, drift_anchor)
        .add_observer(spawn_marker)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.03)));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, -35.0, 35.0).looking_at(Vec3::ZERO, Vec3::Z),
    ));

    let mesh = meshes.add(Mesh::from(Cuboid::new(0.35, 0.35, 0.35)));
    let material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(3.0, 3.5, 4.0),
        ..Default::default()
    });
    commands.insert_resource(Visuals { mesh, material });

    let plan = asset_server.load("simple.scatter");
    let chunk_size = Vec2::splat(CHUNK_SIZE);
    let view_radius = IVec2::splat(VIEW_RADIUS);
    commands.spawn((
        Anchor,
        Transform::default(),
        ScatterStreamSettings::new(plan, chunk_size, view_radius, 12),
    ));
}

fn drift_anchor(time: Res<Time>, mut anchors: Query<&mut Transform, With<Anchor>>) {
    let t = time.elapsed_secs();
    for mut transform in anchors.iter_mut() {
        transform.translation.x = (t * SWAY_SPEED).sin() * SWAY_AMPLITUDE;
        transform.translation.y = t * DRIFT_SPEED;
    }
}

fn spawn_marker(
    event: On<ScatterStreamPlaced>,
    mut commands: Commands,
    visuals: Res<Visuals>,
    mut transforms: Query<&mut Transform>,
) {
    if let Ok(mut transform) = transforms.get_mut(event.entity) {
        transform.scale = Vec3::splat(0.6);
    }
    commands.entity(event.entity).insert((
        Mesh3d(visuals.mesh.clone()),
        MeshMaterial3d(visuals.material.clone()),
    ));
}
