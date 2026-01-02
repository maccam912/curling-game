use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::reflect::TypePath;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct IceMaterial {
    #[uniform(0)]
    pub base_color: LinearRgba,

    #[texture(1)]
    #[sampler(2)]
    pub reflection_texture: Handle<Image>,
}

impl Material for IceMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ice_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
