pub mod render;

pub mod prelude {
    pub use std::{
        fmt::{self, Debug},
        hash::{Hash, Hasher},
        mem::{self, offset_of},
        ops::Range,
    };

    pub use bevy::{
        asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
        camera::visibility::{VisibilityClass, add_visibility_class},
        core_pipeline::{
            core_2d::{CORE_2D_DEPTH_FORMAT, Transparent2d},
            tonemapping::{DebandDither, Tonemapping, TonemappingLuts, get_lut_bind_group_layout_entries, get_lut_bindings},
        },
        ecs::{
            query::ROQueryItem,
            system::{
                SystemParam, SystemParamItem, SystemState,
                lifetimeless::{Read, SRes},
            },
        },
        image::{ImageLoader, ImageLoaderSettings},
        math::{Affine2, FloatOrd},
        mesh::{PrimitiveTopology, VertexBufferLayout, VertexFormat},
        platform::collections::HashMap,
        prelude::*,
        render::{
            Extract, MainWorld, Render, RenderApp, RenderStartup, RenderSystems,
            render_asset::RenderAssets,
            render_phase::{
                AddRenderCommand as _, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult, SetItemPipeline,
                TrackedRenderPass, ViewSortedRenderPhases,
            },
            render_resource::{
                BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendComponent, BlendFactor, BlendOperation,
                BlendState, Buffer, BufferAddress, BufferDescriptor, BufferUsages, COPY_BUFFER_ALIGNMENT, ColorTargetState, ColorWrites,
                CompareFunction, DepthBiasState, DepthStencilState, Extent3d, FragmentState, FrontFace, IndexFormat, MultisampleState, Origin3d,
                PipelineCache, PolygonMode, PrimitiveState, RawBufferVec, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
                SpecializedRenderPipeline, SpecializedRenderPipelines, StencilFaceState, StencilState, TexelCopyBufferLayout, TexelCopyTextureInfo,
                TextureAspect, TextureDimension, TextureFormat, TextureSampleType, VertexAttribute, VertexState, VertexStepMode,
                binding_types::{sampler, texture_2d, uniform_buffer},
            },
            renderer::{RenderDevice, RenderQueue},
            sync_component::SyncComponentPlugin,
            sync_world::RenderEntity,
            texture::{FallbackImage, GpuImage},
            view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        },
        shader::ShaderDefVal,
        sprite::Anchor,
        sprite_render::SpritePipelineKey,
        tasks::ComputeTaskPool,
    };
    pub use bitflags::{bitflags, bitflags_match};
    pub use bytemuck::{Pod, Zeroable, must_cast_slice as cast_slice};
    pub use smallvec::SmallVec;
    pub use vec_belt::{Transfer, VecBelt};
}

use prelude::*;

fn main() -> AppExit {
    App::new().add_plugins((DefaultPlugins, render::plugin)).run()
}
