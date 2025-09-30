//! Texture traits and registry for field inputs.
//!
//! This module defines how external 2D textures integrate into the field graph:
//! - Define custom sources by implementing [`Texture`].
//! - Manage instances with [`TextureRegistry`].
//! - Sample channels via [`TextureChannel`].
use std::collections::HashMap;
use std::sync::Arc;

use glam::Vec2;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Texture channel to sample from.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TextureChannel {
    R,
    G,
    B,
    A,
}

/// Trait for 2D textures sampled at a position in domain/world coordinates.
/// Implementors should map the domain position to their own texel space as needed.
pub trait Texture: Send + Sync {
    fn sample(&self, channel: TextureChannel, p: Vec2) -> f32;
}

/// Registry for storing and managing textures by unique string identifiers.
#[non_exhaustive]
pub struct TextureRegistry {
    textures: HashMap<String, Arc<dyn Texture>>,
}

impl TextureRegistry {
    /// Creates a new, empty [`TextureRegistry`].
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            textures: HashMap::with_capacity(n),
        }
    }

    /// Returns the number of registered textures.
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Returns `true` if there are no registered textures.
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    /// Clears all registered textures.
    pub fn clear(&mut self) {
        self.textures.clear();
    }

    /// Registers a new texture with the given identifier.
    pub fn register<T>(&mut self, id: impl Into<String>, texture: T)
    where
        T: Texture + 'static,
    {
        self.textures.insert(id.into(), Arc::new(texture));
    }

    /// Registers a new texture with the given identifier using an [`Arc`].
    pub fn register_arc(&mut self, id: impl Into<String>, texture: Arc<dyn Texture + 'static>) {
        self.textures.insert(id.into(), texture);
    }

    /// Extends the registry with textures from another registry.
    pub fn extend_from(&mut self, other: &TextureRegistry) {
        for (k, v) in other.textures.iter() {
            self.textures.insert(k.clone(), v.clone());
        }
    }

    /// Unregisters a texture by its identifier. Returns `true` if the texture was found and removed.
    pub fn unregister(&mut self, id: &str) -> bool {
        self.textures.remove(id).is_some()
    }

    /// Checks if a texture with the given identifier exists in the registry.
    pub fn contains(&self, id: &str) -> bool {
        self.textures.contains_key(id)
    }

    /// Retrieves a texture by its identifier, returning an [`Arc`] to the texture if found.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Texture>> {
        self.textures.get(id).cloned()
    }

    /// Samples the specified texture at the given UV coordinates and channel.
    #[inline]
    pub fn sample(&self, texture_id: &str, channel: TextureChannel, p: Vec2) -> f32 {
        if let Some(tex) = self.textures.get(texture_id) {
            tex.sample(channel, p)
        } else {
            warn!("Unknown texture id '{}'.", texture_id);
            0.0
        }
    }
}

impl Default for TextureRegistry {
    fn default() -> Self {
        Self::new()
    }
}
