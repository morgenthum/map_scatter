use bevy::prelude::*;
use bevy_map_scatter::prelude::*;

#[derive(Default, Resource)]
struct PlanHandle(Handle<ScatterPlanAsset>);

fn main() {
    App::new()
        .init_resource::<PlanHandle>()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapScatterPlugin)
        .add_systems(Startup, load_assets)
        .add_systems(Update, trigger_request)
        .add_systems(Update, on_scatter_message)
        .add_observer(print_result)
        .run();
}

/// Loads the scatter plan asset on startup.
fn load_assets(mut asset_handle: ResMut<PlanHandle>, asset_server: Res<AssetServer>) {
    asset_handle.0 = asset_server.load("simple.scatter");
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
    let domain_extent = Vec2::new(10.0, 10.0);

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
    commands.trigger(ScatterRequest::new(entity, handle.0.clone(), config, seed));

    // Mark as done.
    *once = true;
}

fn on_scatter_message(mut reader: MessageReader<ScatterMessage>) {
    for msg in reader.read() {
        let entity = msg.request_entity;

        match &msg.event {
            ScatterEvent::RunStarted { layer_count, .. } => {
                info!(
                    "Scatter run started for entity {:?}: {layer_count} layer(s)",
                    entity
                );
            }
            ScatterEvent::LayerStarted { index, id, .. } => {
                info!("Layer started: #{index} '{id}'");
            }
            ScatterEvent::PositionEvaluated {
                layer_index,
                layer_id,
                position,
                max_weight,
                ..
            } => {
                debug!(
                    "Position evaluated (layer #{}, '{}'): p=({:.2},{:.2}) max_weight={:.3}",
                    layer_index, layer_id, position.x, position.y, max_weight
                );
            }
            ScatterEvent::PlacementMade {
                layer_index,
                layer_id,
                placement,
                ..
            } => {
                info!(
                    "Placement made (layer #{}, '{}'): kind='{}' at ({:.2},{:.2})",
                    layer_index,
                    layer_id,
                    placement.kind_id,
                    placement.position.x,
                    placement.position.y
                );
            }
            ScatterEvent::OverlayGenerated {
                layer_index,
                layer_id,
                summary,
            } => {
                info!(
                    "Overlay generated (layer #{}, '{}'): name='{}' size={:?}",
                    layer_index, layer_id, summary.name, summary.size_px
                );
            }
            ScatterEvent::LayerFinished {
                index, id, result, ..
            } => {
                info!(
                    "Layer finished: #{index} '{id}', placements={}",
                    result.placements.len()
                );
            }
            ScatterEvent::RunFinished { result } => {
                info!(
                    "Scatter run finished for entity {:?}: placements={}, evaluated={}, rejected={}",
                    entity,
                    result.placements.len(),
                    result.positions_evaluated,
                    result.positions_rejected
                );
            }
            ScatterEvent::Warning { context, message } => {
                warn!("Warning '{}': {}", context, message);
            }
            _ => {}
        }
    }
}

/// Observes the `EntityEvent` when a scatter run has finished.
fn print_result(finished: On<ScatterFinished>, mut commands: Commands) {
    info!(
        "Scatter run {} finished: placements={}, evaluated={}, rejected={}",
        finished.entity,
        finished.result.placements.len(),
        finished.result.positions_evaluated,
        finished.result.positions_rejected
    );

    // Clean up the entity used for the request.
    commands.entity(finished.entity).despawn();
}
