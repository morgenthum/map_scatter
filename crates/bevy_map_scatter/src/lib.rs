//! Bevy plugin for map_scatter providing assets, resources, message types, and systems.
#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

pub use assets::{
    SamplingDef, ScatterKindDef, ScatterLayerDef, ScatterPlanAsset, ScatterPlanAssetLoader,
    SelectionStrategyDef,
};
use bevy::prelude::*;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
pub use events::{ChannelSink, ScatterBus, ScatterMessage};
use map_scatter::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
pub use textures::ImageTexture;

mod assets;
mod events;
mod textures;

/// Convenient re-exports for common types. Import with `use bevy_map_scatter::prelude::*;`.
pub mod prelude {
    pub use map_scatter::prelude::*;

    pub use crate::assets::{
        ParentDef, SamplingDef, ScatterKindDef, ScatterLayerDef, ScatterPlanAsset,
        ScatterPlanAssetLoader, SelectionStrategyDef,
    };
    pub use crate::events::{ChannelSink, ScatterBus, ScatterMessage};
    pub use crate::textures::ImageTexture;
    pub use crate::{MapScatterPlugin, ScatterFinished, ScatterRequest, ScatterTextureRegistry};
}

/// Bevy plugin providing assets, resources, message types, and systems.
pub struct MapScatterPlugin;

/// Shared texture registry (read-only) used by all runs.
/// Register your textures at startup or via custom systems.
#[derive(Resource, Clone)]
pub struct ScatterTextureRegistry(pub Arc<TextureRegistry>);

impl Default for ScatterTextureRegistry {
    fn default() -> Self {
        Self(Arc::new(TextureRegistry::new()))
    }
}

/// Shared field program cache. It is protected by a mutex to allow async jobs to reuse programs.
#[derive(Resource, Clone)]
struct ScatterCache(pub Arc<Mutex<FieldProgramCache>>);

impl Default for ScatterCache {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(FieldProgramCache::new())))
    }
}

/// A request to run a scatter plan (by asset handle) with a configuration and RNG seed.
#[derive(EntityEvent)]
pub struct ScatterRequest {
    pub entity: Entity,
    pub plan: Handle<ScatterPlanAsset>,
    pub config: RunConfig,
    pub seed: u64,
}

impl ScatterRequest {
    pub fn new(
        entity: Entity,
        plan: Handle<ScatterPlanAsset>,
        config: RunConfig,
        seed: u64,
    ) -> Self {
        Self {
            entity,
            plan,
            config,
            seed,
        }
    }
}

/// Component holding an async scatter job task.
/// This is added to entities with a [`ScatterRequest`] when a job is spawned.
#[derive(Component)]
struct ScatterJob {
    pub task: Option<Task<RunResult>>,
}

/// [`EntityEvent`] triggered when a scatter run has finished.
#[derive(EntityEvent, Clone)]
pub struct ScatterFinished {
    pub entity: Entity,
    pub result: RunResult,
}

impl Plugin for MapScatterPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ScatterMessage>()
            .init_asset::<ScatterPlanAsset>()
            .init_asset_loader::<ScatterPlanAssetLoader>()
            .init_resource::<ScatterBus>()
            .init_resource::<ScatterTextureRegistry>()
            .init_resource::<ScatterCache>()
            .add_systems(Update, poll_scatter_jobs)
            .add_systems(Update, drain_scatter_messages)
            .add_observer(spawn_scatter_job);
    }
}

fn spawn_scatter_job(
    request: On<ScatterRequest>,
    mut commands: Commands,
    bus: Res<ScatterBus>,
    cache: Res<ScatterCache>,
    textures: Res<ScatterTextureRegistry>,
    assets: Res<Assets<ScatterPlanAsset>>,
) {
    let pool = AsyncComputeTaskPool::get();
    let tx = bus.as_ref().tx.clone();
    let entity = request.entity;

    let Some(plan) = assets.get(&request.plan) else {
        error!("ScatterPlanAsset not loaded yet: {:?}", request.plan);
        return;
    };

    // Prepare data for the task
    let plan = plan.into();
    let config = request.config.clone();
    let seed = request.seed;
    let textures = textures.0.clone();
    let cache = cache.0.clone();
    let tx = tx.clone();

    // Spawn async job returning the RunResult
    let task = pool.spawn(async move {
        let mut rng = StdRng::seed_from_u64(seed);

        // Stream events through channel sink
        let mut sink = ChannelSink {
            request: entity,
            tx,
        };

        // Use cache with a short-lived lock for runner lifetime
        let mut cache_guard = cache.lock().expect("ScatterCache mutex poisoned");
        let mut runner = ScatterRunner::new(config.clone(), &textures, &mut cache_guard);
        let result = runner.run_with_events(&plan, &mut rng, &mut sink);
        drop(cache_guard);

        result
    });

    // Attach job component to the entity
    commands
        .entity(request.entity)
        .insert(ScatterJob { task: Some(task) });
}

fn poll_scatter_jobs(mut commands: Commands, mut job_query: Query<(Entity, &mut ScatterJob)>) {
    for (entity, mut job) in job_query.iter_mut() {
        if let Some(task) = job.task.take() {
            if task.is_finished() {
                let result = block_on(task).clone();

                // Remove job component when done.
                commands.entity(entity).remove::<ScatterJob>();

                // Trigger finished `EntityEvent`.
                commands.trigger(ScatterFinished { entity, result });
            } else {
                job.task = Some(task);
            }
        }
    }
}

fn drain_scatter_messages(
    mut bus: ResMut<ScatterBus>,
    mut messages: ResMut<Messages<ScatterMessage>>,
) {
    let bus = bus.as_mut();
    while let Ok(message) = bus.rx.try_recv() {
        messages.write(message);
    }
}
