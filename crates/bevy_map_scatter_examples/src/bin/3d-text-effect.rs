use std::sync::Arc;

use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_map_scatter::prelude::*;
use rand::Rng;

/// Resource to hold asset handles
#[derive(Default, Resource)]
struct Handles {
    plan: Handle<ScatterPlanAsset>,
    image: Handle<Image>,
}

/// Component to apply a cool effect to the cubes
#[derive(Component)]
struct Cube {
    base_height: f32,
    amplitude: f32,
    speed: f32,
    phase: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapScatterPlugin)
        .add_systems(Startup, (setup_camera, load_assets))
        .add_systems(Startup, load_assets)
        .add_systems(Update, (trigger_request, randomize_scale))
        .add_observer(on_scatter_finished)
        .run();
}

/// Setup a simple 3D camera with bloom,
/// which helps to see the emissive material.
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Bloom::NATURAL,
        Transform::from_xyz(0.0, 400.0, 100.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));
}

/// Load assets on startup.
fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Handles {
        plan: asset_server.load("mask.scatter"),
        image: asset_server.load("no_space.png"),
    });
}

/// Triggers a scatter request once the plan asset is loaded.
fn trigger_request(
    mut commands: Commands,
    mut once: Local<bool>,
    mut registry: ResMut<ScatterTextureRegistry>,
    plans: Res<Assets<ScatterPlanAsset>>,
    images: Res<Assets<Image>>,
    handles: Res<Handles>,
) {
    // Only run once.
    if *once {
        return;
    }
    // Wait until the plan and image assets are both loaded.
    if plans.get(&handles.plan).is_none() {
        return;
    }
    // Wait until the image asset is loaded.
    let Some(image) = images.get(handles.image.id()) else {
        return;
    };

    // The domain size for scattering.
    let domain_extent = Vec2::new(500.0, 500.0);

    // Create and register texture from image asset.
    let texture = ImageTexture::from_image(image, domain_extent).expect("texture from image");

    let mut reg = TextureRegistry::new();
    reg.register("no_space", texture);
    *registry = ScatterTextureRegistry(Arc::new(reg));

    // Create run configuration and seed for (deterministic) randomness.
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(domain_extent.x.max(domain_extent.y))
        .with_raster_cell_size(1.0);
    let seed = 42;

    // Spawn an entity to track the request.
    // In real applications you might want to add your own components here,
    // or use an existing entity.
    let entity = commands.spawn_empty().id();

    // Trigger the scatter run.
    commands.trigger(ScatterRequest::new(
        entity,
        handles.plan.clone(),
        config,
        seed,
    ));

    // Mark as done.
    *once = true;
}

/// Observes the `EntityEvent` when a scatter run has finished and spawns actual 3D entities.
fn on_scatter_finished(
    finished: On<ScatterFinished>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create shared mesh and material
    let mesh = meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
    let material = materials.add(StandardMaterial {
        emissive: LinearRgba::rgb(100.0, 0.0, 150.0),
        ..Default::default()
    });

    let mut rng = rand::rng();

    // Spawn a cube for each placement
    for placement in &finished.result.placements {
        let pos2 = placement.position;
        let translation = Vec3::new(pos2.x, 0.0, pos2.y);

        // Apply cool effect
        let x_scale = rng.random_range(0.8..=1.2);
        let z_scale = rng.random_range(0.8..=1.2);
        let base_height = rng.random_range(10.0..=25.0);
        let amplitude = rng.random_range(2.0..=8.0);
        let speed = rng.random_range(2.0..=10.0);
        let phase = rng.random_range(0.0..=std::f32::consts::TAU);

        let scale = vec3(x_scale, base_height + amplitude * phase.sin(), z_scale);

        // Spawn the actual 3d entity
        commands.spawn((
            Name::new("Cube"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform {
                translation,
                scale,
                ..default()
            },
            Cube {
                base_height,
                amplitude,
                speed,
                phase,
            },
        ));
    }

    // Clean up the entity used for the request.
    commands.entity(finished.entity).despawn();
}

/// Changes the scale of the cubes over time to create a effect.
fn randomize_scale(time: Res<Time>, mut query: Query<(&Cube, &mut Transform)>) {
    let t = time.elapsed_secs();

    for (cube, mut transform) in query.iter_mut() {
        let new_y = cube.base_height + cube.amplitude * (cube.phase + t * cube.speed).sin();
        transform.scale.y = new_y.max(0.0);
    }
}
