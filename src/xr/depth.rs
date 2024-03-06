use std::ptr::{null, null_mut};

use ash::vk::{Handle, Image};
use bevy::{
    core_pipeline::core_3d::{
        graph::{Core3d, Node3d},
        prepare_core_3d_depth_textures, Core3dPlugin,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            BindGroupLayout, CachedRenderPipelineId, FragmentState, PipelineCache,
            RenderPipelineDescriptor, Texture,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewDepthTexture,
        Extract, Render, RenderSet,
    },
};
use bevy_oxr::{
    input::XrInput,
    resources::{XrFrameState, XrInstance, XrSession},
    xr::{
        self,
        raw::EnvironmentDepthMETA,
        sys::{
            EnvironmentDepthImageAcquireInfoMETA, EnvironmentDepthImageMETA,
            EnvironmentDepthImageViewMETA, EnvironmentDepthProviderCreateInfoMETA,
            EnvironmentDepthProviderMETA, EnvironmentDepthSwapchainCreateInfoMETA,
            EnvironmentDepthSwapchainMETA, EnvironmentDepthSwapchainStateMETA,
            SwapchainImageVulkanKHR,
        },
        EnvironmentDepthProviderCreateFlagsMETA, Fovf, Posef, StructureType,
    },
    xr_input::xr_camera::{Eye, XrCameraType},
};
use wgpu::{
    hal::{MemoryFlags, TextureUses},
    BindGroupEntry, BindGroupLayoutEntry, BindingResource, DepthBiasState, DepthStencilState,
    RenderPassDescriptor, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

use crate::oxr;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, RenderLabel)]
struct DepthRenderNode;

pub struct EnvDepthPlugin;

impl bevy::prelude::Plugin for EnvDepthPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let instance: &XrInstance = app.world.resource();
        let session: &XrSession = app.world.resource();
        let vtable = instance.exts().meta_environment_depth.unwrap();
        let mut xr_provider = EnvironmentDepthProviderMETA::NULL;
        let info = EnvironmentDepthProviderCreateInfoMETA {
            ty: StructureType::ENVIRONMENT_DEPTH_PROVIDER_CREATE_INFO_META,
            next: null(),
            create_flags: EnvironmentDepthProviderCreateFlagsMETA::EMPTY,
        };
        oxr!((vtable.create_environment_depth_provider)(
            session.as_raw(),
            &info,
            &mut xr_provider,
        ));

        let mut xr_swapchain = EnvironmentDepthSwapchainMETA::NULL;
        let info = EnvironmentDepthSwapchainCreateInfoMETA {
            ty: StructureType::ENVIRONMENT_DEPTH_SWAPCHAIN_CREATE_INFO_META,
            next: null(),
            create_flags: Default::default(),
        };
        oxr!((vtable.create_environment_depth_swapchain)(
            xr_provider,
            &info as _,
            &mut xr_swapchain as _,
        ));

        let mut state = EnvironmentDepthSwapchainStateMETA {
            ty: StructureType::ENVIRONMENT_DEPTH_SWAPCHAIN_STATE_META,
            next: null_mut(),
            width: 0,
            height: 0,
        };
        oxr!((vtable.get_environment_depth_swapchain_state)(
            xr_swapchain,
            &mut state as _
        ));

        let mut n_images = 0u32;
        oxr!((vtable.enumerate_environment_depth_swapchain_images)(
            xr_swapchain,
            0,
            &mut n_images as _,
            null_mut()
        ));
        let mut images = std::iter::repeat(SwapchainImageVulkanKHR {
            ty: StructureType::SWAPCHAIN_IMAGE_VULKAN_KHR,
            next: null_mut(),
            image: 0,
        })
        .take(n_images as usize)
        .collect::<Vec<_>>();
        oxr!((vtable.enumerate_environment_depth_swapchain_images)(
            xr_swapchain,
            n_images,
            &mut n_images as _,
            images.as_mut_ptr().cast(),
        ));

        let device: &RenderDevice = app.world.resource();
        let wgpu_device = device.wgpu_device();
        let swapchain = images
            .into_iter()
            .map(|i| unsafe {
                let size = wgpu::Extent3d {
                    width: state.width,
                    height: state.height,
                    depth_or_array_layers: 2,
                };
                let hal = wgpu::hal::vulkan::Device::texture_from_raw(
                    Image::from_raw(i.image),
                    &wgpu::hal::TextureDescriptor {
                        label: None,
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Depth16Unorm,
                        usage: TextureUses::DEPTH_STENCIL_READ,
                        memory_flags: MemoryFlags::empty(),
                        view_formats: Vec::new(),
                    },
                    None,
                );
                let wgpu = wgpu_device.create_texture_from_hal::<wgpu::hal::vulkan::Api>(
                    hal,
                    &TextureDescriptor {
                        label: None,
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Depth16Unorm,
                        usage: TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                );
                Texture::from(wgpu)
            })
            .collect();

        oxr!((vtable.start_environment_depth_provider)(xr_provider));

        let bg_layout = device.create_bind_group_layout(
            None,
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        );

        let mut shaders = app.world.resource_mut::<Assets<Shader>>();
        let shader = shaders.add(Shader::from_wgsl(
            include_str!("depth.wgsl"),
            "src/depth.wgsl",
        ));
        let render_app = app.sub_app_mut(bevy::render::RenderApp);
        let pipeline_cache = render_app.world.resource_mut::<PipelineCache>();
        let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: None,
            layout: vec![bg_layout.clone()],
            push_constant_ranges: vec![],
            vertex: bevy::core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state(),
            primitive: default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "main_fs".into(),
                targets: vec![],
            }),
        });

        render_app.add_systems(ExtractSchedule, extract_eyes);
        // this has to happen once per frame, we can't do it in the render graph which runs per-eye
        render_app.add_systems(
            Render,
            acquire_depth_image
                .after(prepare_core_3d_depth_textures)
                .in_set(RenderSet::PrepareResources),
        );
        render_app.insert_resource(AcquiredImage(None));

        render_app.insert_resource(EnvDepth {
            vtable,
            xr_provider,
            swapchain,
            bg_layout,
            pipeline,
        });
    }

    fn ready(&self, app: &App) -> bool {
        app.is_plugin_added::<Core3dPlugin>()
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(bevy::render::RenderApp);
        let name = DepthRenderNode;
        render_app.add_render_graph_node::<ViewNodeRunner<EnvDepthRenderNode>>(Core3d, name);
        render_app.add_render_graph_edges(Core3d, (name, Node3d::Prepass));
    }
}

fn extract_eyes(mut cmds: Commands, query: Extract<Query<(Entity, &XrCameraType)>>) {
    for (id, cam_ty) in query.iter() {
        cmds.get_or_spawn(id).insert(cam_ty.clone());
    }
}

#[derive(Resource)]
struct EnvDepth {
    vtable: EnvironmentDepthMETA,
    xr_provider: EnvironmentDepthProviderMETA,
    swapchain: Vec<Texture>,
    bg_layout: BindGroupLayout,
    pipeline: CachedRenderPipelineId,
}

#[derive(Resource)]
struct AcquiredImage(Option<EnvironmentDepthImageMETA>);
// shut up
unsafe impl Send for AcquiredImage {}
unsafe impl Sync for AcquiredImage {}

fn acquire_depth_image(
    xr_input: Res<XrInput>,
    frame_state: Res<XrFrameState>,
    d: ResMut<EnvDepth>,
    mut i: ResMut<AcquiredImage>,
) {
    let info = EnvironmentDepthImageAcquireInfoMETA {
        ty: StructureType::ENVIRONMENT_DEPTH_IMAGE_ACQUIRE_INFO_META,
        next: null(),
        space: xr_input.head.as_raw(),
        display_time: frame_state.lock().unwrap().predicted_display_time,
    };
    let mut image = EnvironmentDepthImageMETA {
        ty: StructureType::ENVIRONMENT_DEPTH_IMAGE_META,
        next: null(),
        swapchain_index: 0,
        near_z: 0.,
        far_z: 0.,
        views: [EnvironmentDepthImageViewMETA {
            ty: StructureType::ENVIRONMENT_DEPTH_IMAGE_VIEW_META,
            next: null(),
            fov: Fovf::default(),
            pose: Posef::default(),
        }; 2],
    };
    let res =
        unsafe { (d.vtable.acquire_environment_depth_image)(d.xr_provider, &info, &mut image) };
    if res != xr::sys::Result::SUCCESS {
        *i = AcquiredImage(None);
    }

    *i = AcquiredImage(Some(image));
}

struct EnvDepthRenderNode;
impl FromWorld for EnvDepthRenderNode {
    fn from_world(_: &mut World) -> Self {
        EnvDepthRenderNode
    }
}
impl ViewNode for EnvDepthRenderNode {
    type ViewQuery = (&'static ViewDepthTexture, &'static XrCameraType);

    fn run(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (depth_texture, eye): QueryItem<Self::ViewQuery>,
        w: &World,
    ) -> Result<(), NodeRunError> {
        let Some(image) = w.resource::<AcquiredImage>().0 else {
            return Ok(());
        };
        let d = w.resource::<EnvDepth>();
        let Some(pipeline) = w
            .resource::<PipelineCache>()
            .get_render_pipeline(d.pipeline)
        else {
            return Ok(());
        };
        let eye = match eye {
            XrCameraType::Xr(Eye::Left) => 0,
            XrCameraType::Xr(Eye::Right) => 1,
            XrCameraType::Flatscreen => panic!("what?"),
        };
        let device = render_context.render_device().clone();

        let tex_view =
            d.swapchain[image.swapchain_index as usize].create_view(&TextureViewDescriptor {
                label: None,
                format: Some(TextureFormat::Depth16Unorm),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: eye,
                array_layer_count: Some(1),
            });
        let bg = device.create_bind_group(
            None,
            &d.bg_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&tex_view),
            }],
        );

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(depth_texture.get_attachment(wgpu::StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_bind_group(0, &bg, &[]);
        pass.set_render_pipeline(pipeline);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
