//TODO don't move these imports into `main.rs`, because 0.19 will use a Schedule-based render graph
// with a different API
use bevy::{
    core_pipeline::{
        FullscreenShader,
        core_3d::graph::{Core3d, Node3d},
    },
    render::{
        render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt as _, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            BindGroupEntries, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, MultisampleState,
            RenderPassColorAttachment, RenderPassDescriptor, TextureSampleType, binding_types::texture_2d,
        },
        renderer::RenderContext,
    },
};

use crate::{camera::PrimaryCamera, prelude::*};

pub(super) fn plugin(app: &mut App) {
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .add_render_graph_node::<ViewNodeRunner<PixelizationNode>>(Core3d, PixelizationLabel)
            .add_render_graph_edges(Core3d, (Node3d::Tonemapping, PixelizationLabel, Node3d::EndMainPassPostProcessing))
            .add_systems(RenderStartup, init_pixelization_pipeline);
    }
}

#[derive(Resource, Debug)]
pub struct PixelizationPipeline {
    pub layout: BindGroupLayoutDescriptor,
    pub id: CachedRenderPipelineId,
}

pub fn init_pixelization_pipeline(
    mut commands: Commands,
    server: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "pixelation_bind_group_layout",
        &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (texture_2d(TextureSampleType::Float { filterable: false }),)),
    );

    let shader = server.load("shaders/post_process/pixelization.wgsl");
    let vertex = fullscreen.to_vertex_state();
    let id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("pixelization_pipeline".into()),
        layout: vec![layout.clone()],
        vertex,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        multisample: MultisampleState {
            count: 4,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        zero_initialize_workgroup_memory: false,
        ..default()
    });

    commands.insert_resource(PixelizationPipeline { layout, id });
}

#[derive(RenderLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelizationLabel;

#[derive(Default)]
pub struct PixelizationNode;
impl ViewNode for PixelizationNode {
    type ViewQuery = (Read<PrimaryCamera>, Read<ViewTarget>);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (.., target): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<PixelizationPipeline>();
        let cache = world.resource::<PipelineCache>();

        let Some(render_pipeline) = cache.get_render_pipeline(pipeline.id) else { return Ok(()) };
        let post_process = target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "pixelization_bind_group",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((post_process.source,)),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("pixelization_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target.sampled_main_texture_view().unwrap(),
                depth_slice: None,
                resolve_target: Some(post_process.destination),
                ops: default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}
