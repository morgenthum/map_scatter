use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use map_scatter::fieldgraph::ChunkId;
use map_scatter::prelude::{seed_for_chunk, Placement, RunConfig};

use crate::{ScatterFinished, ScatterPlanAsset, ScatterRequest};

/// Settings for streaming scatter chunks around an anchor entity.
#[derive(Component, Clone)]
pub struct ScatterStreamSettings {
    /// Scatter plan asset to execute per chunk.
    pub plan: Handle<ScatterPlanAsset>,
    /// Chunk size in world units.
    pub chunk_size: Vec2,
    /// View radius (in chunks) around the anchor.
    pub view_radius: IVec2,
    /// Base RNG seed for chunk seeding.
    pub seed: u64,
    /// Chunk extent used for evaluation in world units.
    pub chunk_extent: f32,
    /// Raster cell size used for field sampling.
    pub raster_cell_size: f32,
    /// Halo cell count used for chunked evaluation.
    pub grid_halo: usize,
    /// Offset applied to the anchor position when choosing the focus.
    pub focus_offset: Vec2,
}

impl ScatterStreamSettings {
    pub fn new(
        plan: Handle<ScatterPlanAsset>,
        chunk_size: Vec2,
        view_radius: IVec2,
        seed: u64,
    ) -> Self {
        let chunk_extent = chunk_size.x.max(chunk_size.y);
        Self {
            plan,
            chunk_size,
            view_radius,
            seed,
            chunk_extent,
            raster_cell_size: 1.0,
            grid_halo: 2,
            focus_offset: Vec2::ZERO,
        }
    }

    pub fn with_chunk_extent(mut self, chunk_extent: f32) -> Self {
        self.chunk_extent = chunk_extent;
        self
    }

    pub fn with_raster_cell_size(mut self, raster_cell_size: f32) -> Self {
        self.raster_cell_size = raster_cell_size;
        self
    }

    pub fn with_grid_halo(mut self, grid_halo: usize) -> Self {
        self.grid_halo = grid_halo;
        self
    }

    pub fn with_focus_offset(mut self, focus_offset: Vec2) -> Self {
        self.focus_offset = focus_offset;
        self
    }
}

/// Chunk tracking for streaming state on an anchor entity.
#[derive(Component, Default)]
pub struct ScatterStreamChunks(
    /// Map from chunk id to spawned chunk entity.
    pub HashMap<IVec2, Entity>,
);

/// Component added to each spawned chunk root.
#[derive(Component, Debug, Clone)]
pub struct ScatterStreamChunk {
    /// Anchor entity that owns this chunk.
    pub anchor: Entity,
    /// Chunk id in the stream grid.
    pub id: IVec2,
    /// World-space center of the chunk.
    pub center: Vec2,
}

/// Component added to each spawned placement entity.
#[derive(Component, Debug, Clone)]
pub struct ScatterStreamPlacement {
    /// Kind identifier for this placement.
    pub kind_id: String,
    /// World-space position of the placement.
    pub world_position: Vec2,
}

/// [`EntityEvent`] emitted when a streamed placement entity is spawned.
#[derive(EntityEvent, Debug, Clone)]
pub struct ScatterStreamPlaced {
    /// Entity spawned for the placement.
    pub entity: Entity,
    /// Chunk entity that owns the placement.
    pub chunk_entity: Entity,
    /// Chunk id that produced the placement.
    pub chunk_id: IVec2,
    /// Placement data from the scatter run.
    pub placement: Placement,
}

/// Plugin for streaming scatter chunks around anchor entities (requires [`MapScatterPlugin`]).
pub struct MapScatterStreamingPlugin;

impl Plugin for MapScatterStreamingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_streams)
            .add_observer(handle_scatter_finished);
    }
}

fn update_streams(
    mut commands: Commands,
    assets: Res<Assets<ScatterPlanAsset>>,
    mut anchors: Query<(
        Entity,
        &GlobalTransform,
        &ScatterStreamSettings,
        Option<&mut ScatterStreamChunks>,
    )>,
) {
    for (anchor_entity, transform, settings, chunks_opt) in anchors.iter_mut() {
        let Some(mut chunks) = chunks_opt else {
            commands
                .entity(anchor_entity)
                .insert(ScatterStreamChunks::default());
            continue;
        };

        if assets.get(&settings.plan).is_none() {
            continue;
        }

        if settings.chunk_size.x <= 0.0 || settings.chunk_size.y <= 0.0 {
            warn!(
                "ScatterStreamSettings chunk_size must be > 0 (got {:?}).",
                settings.chunk_size
            );
            continue;
        }

        let focus = transform.translation().truncate() + settings.focus_offset;
        let center_chunk = world_to_chunk_id_centered(focus, settings.chunk_size);
        let view = IVec2::new(settings.view_radius.x.max(0), settings.view_radius.y.max(0));

        let mut desired = HashSet::new();
        for dy in -view.y..=view.y {
            for dx in -view.x..=view.x {
                desired.insert(center_chunk + IVec2::new(dx, dy));
            }
        }

        let mut to_remove = Vec::new();
        for (&chunk_id, &entity) in chunks.0.iter() {
            if !desired.contains(&chunk_id) {
                to_remove.push(chunk_id);
                commands.entity(entity).despawn();
            }
        }
        for chunk_id in to_remove {
            chunks.0.remove(&chunk_id);
        }

        for chunk_id in desired {
            if chunks.0.contains_key(&chunk_id) {
                continue;
            }

            let center = chunk_center(chunk_id, settings.chunk_size);
            let config = RunConfig::new(settings.chunk_size)
                .with_domain_center(center)
                .with_chunk_extent(settings.chunk_extent)
                .with_raster_cell_size(settings.raster_cell_size)
                .with_grid_halo(settings.grid_halo);

            if let Err(err) = config.validate() {
                warn!("Scatter stream config invalid for {:?}: {}", chunk_id, err);
                continue;
            }

            let chunk_entity = commands
                .spawn((
                    ScatterStreamChunk {
                        anchor: anchor_entity,
                        id: chunk_id,
                        center,
                    },
                    Transform::from_translation(center.extend(0.0)),
                ))
                .id();

            chunks.0.insert(chunk_id, chunk_entity);

            let seed = seed_for_chunk(settings.seed, ChunkId(chunk_id.x, chunk_id.y));
            commands.trigger(ScatterRequest::new(
                chunk_entity,
                settings.plan.clone(),
                config,
                seed,
            ));
        }
    }
}

fn handle_scatter_finished(
    finished: On<ScatterFinished>,
    mut commands: Commands,
    chunks: Query<&ScatterStreamChunk>,
) {
    let Ok(chunk) = chunks.get(finished.entity) else {
        return;
    };

    let center = chunk.center;
    let mut placed_events = Vec::with_capacity(finished.result.placements.len());
    commands.entity(finished.entity).with_children(|parent| {
        for placement in &finished.result.placements {
            let local = placement.position - center;
            let entity = parent
                .spawn((
                    ScatterStreamPlacement {
                        kind_id: placement.kind_id.clone(),
                        world_position: placement.position,
                    },
                    Transform::from_translation(Vec3::new(local.x, local.y, 0.0)),
                ))
                .id();
            placed_events.push(ScatterStreamPlaced {
                entity,
                chunk_entity: finished.entity,
                chunk_id: chunk.id,
                placement: placement.clone(),
            });
        }
    });

    for event in placed_events {
        commands.trigger(event);
    }
}

fn world_to_chunk_id_centered(pos: Vec2, chunk_size: Vec2) -> IVec2 {
    let x = ((pos.x / chunk_size.x) + 0.5).floor() as i32;
    let y = ((pos.y / chunk_size.y) + 0.5).floor() as i32;
    IVec2::new(x, y)
}

fn chunk_center(id: IVec2, chunk_size: Vec2) -> Vec2 {
    Vec2::new(id.x as f32 * chunk_size.x, id.y as f32 * chunk_size.y)
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    fn setup_app() -> (App, Entity, Vec2) {
        let mut app = App::new();
        app.add_systems(Update, update_streams);

        let mut assets = Assets::<ScatterPlanAsset>::default();
        let plan = assets.add(ScatterPlanAsset { layers: Vec::new() });
        app.world_mut().insert_resource(assets);

        let chunk_size = Vec2::splat(10.0);
        let view_radius = IVec2::ZERO;
        let anchor = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                ScatterStreamSettings::new(plan, chunk_size, view_radius, 1),
            ))
            .id();

        (app, anchor, chunk_size)
    }

    #[test]
    fn spawns_initial_chunk_and_tracks_state() {
        let (mut app, anchor, _chunk_size) = setup_app();

        app.update();
        assert!(app.world().get::<ScatterStreamChunks>(anchor).is_some());

        app.update();

        let chunks = app.world().get::<ScatterStreamChunks>(anchor).unwrap();
        assert_eq!(chunks.0.len(), 1);
        assert!(chunks.0.contains_key(&IVec2::ZERO));

        let chunk_entity = chunks.0[&IVec2::ZERO];
        let chunk = app.world().get::<ScatterStreamChunk>(chunk_entity).unwrap();
        assert_eq!(chunk.anchor, anchor);
        assert_eq!(chunk.id, IVec2::ZERO);
        assert_eq!(chunk.center, Vec2::ZERO);

        let transform = app.world().get::<Transform>(chunk_entity).unwrap();
        assert_eq!(transform.translation, Vec3::ZERO);
    }

    #[test]
    fn replaces_chunks_when_anchor_moves() {
        let (mut app, anchor, chunk_size) = setup_app();

        app.update();
        app.update();

        let chunks = app.world().get::<ScatterStreamChunks>(anchor).unwrap();
        let old_chunk_entity = chunks.0[&IVec2::ZERO];

        app.world_mut()
            .entity_mut(anchor)
            .insert(GlobalTransform::from(Transform::from_translation(
                Vec3::new(chunk_size.x, 0.0, 0.0),
            )));

        app.update();

        let chunks = app.world().get::<ScatterStreamChunks>(anchor).unwrap();
        assert_eq!(chunks.0.len(), 1);
        assert!(chunks.0.contains_key(&IVec2::new(1, 0)));

        let new_chunk_entity = chunks.0[&IVec2::new(1, 0)];
        let chunk = app
            .world()
            .get::<ScatterStreamChunk>(new_chunk_entity)
            .unwrap();
        assert_eq!(chunk.center, Vec2::new(chunk_size.x, 0.0));
        assert!(app.world().get_entity(old_chunk_entity).is_err());
    }
}
