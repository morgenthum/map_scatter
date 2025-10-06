use bevy::prelude::*;
use bevy_map_scatter::prelude::*;

#[derive(Resource, Default)]
struct PlanHandle(Handle<ScatterPlanAsset>);

fn main() {
    App::new()
        .init_resource::<PlanHandle>()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapScatterPlugin)
        .add_systems(Startup, load_plan)
        .add_systems(Update, trigger_request)
        .add_observer(log_finished)
        .run();
}

/// Loads the scatter plan asset on startup.
fn load_plan(mut handle: ResMut<PlanHandle>, assets: Res<AssetServer>) {
    handle.0 = assets.load("simple.scatter");
}

/// Triggers a scatter request once the plan asset is loaded.
fn trigger_request(
    mut commands: Commands,
    mut once: Local<bool>,
    handle: Res<PlanHandle>,
    assets: Res<Assets<ScatterPlanAsset>>,
) {
    // Only run once.
    if *once {
        return;
    }
    // Wait until the asset is loaded.
    if assets.get(&handle.0).is_none() {
        return;
    }

    // The domain size for scattering.
    let domain = Vec2::new(10.0, 10.0);

    // Create run configuration and seed for (deterministic) randomness.
    let config = RunConfig::new(domain)
        .with_chunk_extent(domain.x)
        .with_raster_cell_size(1.0);

    // Spawn an entity to track the request.
    // In real applications you might want to add your own components here,
    // or use an existing entity.
    let entity = commands.spawn_empty().id();

    // Trigger the scatter run.
    commands.trigger(ScatterRequest::new(entity, handle.0.clone(), config, 42));

    // Mark as done.
    *once = true;
}

/// Observes the `EntityEvent` when a scatter run has finished.
fn log_finished(finished: On<ScatterFinished>, mut commands: Commands) {
    info!(
        "Scatter run {} finished: placements={} evaluated={} rejected={}",
        finished.entity,
        finished.result.placements.len(),
        finished.result.positions_evaluated,
        finished.result.positions_rejected
    );

    // Clean up the entity used for the request.
    // In real applications you might want to keep it,
    // depending on your use case.
    commands.entity(finished.entity).despawn();
}
