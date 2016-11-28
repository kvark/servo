/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![crate_name = "webmetal"]
#![crate_type = "rlib"]
#![feature(plugin)]
#![feature(proc_macro)]
#![plugin(plugins)]

extern crate glsl_to_spirv;
#[macro_use] extern crate serde_derive;
extern crate shared_library;
extern crate vk_sys as vk;

mod command;
mod device;

pub use self::command::{CommandBuffer, Queue};
pub use self::device::{Device, DeviceMapper};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct WebMetalCapabilities;

pub struct Share {
    vk: vk::DevicePointers,
}

pub struct ResourceState {
    image_layouts: HashMap<(Arc<Texture>, u32), vk::ImageLayout>,
}

impl ResourceState {
    pub fn new() -> ResourceState {
        ResourceState {
            image_layouts: HashMap::new(),
        }
    }
}

pub type RenderPassClearValues = Vec<vk::ClearValue>;
pub type RenderPassKey = vk::RenderPass;

#[derive(Clone, Debug, Hash, Deserialize, Serialize)]
pub struct RenderPass {
    inner: vk::RenderPass,
    num_colors: usize,
}

impl RenderPass {
    pub fn new(inner: vk::RenderPass, ncol: usize) -> RenderPass {
        RenderPass {
            inner: inner,
            num_colors: ncol,
        }
    }

    pub fn get_inner(&self) -> vk::RenderPass {
        self.inner
    }

    pub fn get_num_colors(&self) -> usize {
        self.num_colors
    }
}

pub struct FrameBuffer {
    inner: vk::Framebuffer,
    extent: vk::Extent2D,
}

impl FrameBuffer {
    pub fn new(inner: vk::Framebuffer, dim: Dimensions) -> FrameBuffer {
        FrameBuffer {
            inner: inner,
            extent: vk::Extent2D {
                width: dim.w,
                height: dim.h,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Dimensions {
    pub w: u32,
    pub h: u32,
    pub d: u32,
}

impl From<vk::Extent3D> for Dimensions {
    fn from(ext: vk::Extent3D) -> Dimensions {
        Dimensions {
            w: ext.width,
            h: ext.height,
            d: ext.depth,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Texture {
    inner: vk::Image,
    memory: vk::DeviceMemory,
    default_layout: vk::ImageLayout,
    dim: Dimensions,
    usage: vk::ImageUsageFlagBits,
    format: vk::Format,
    samples: vk::SampleCountFlagBits,
}

impl Texture {
    fn get_layer_size(&self) -> u32 {
        let bpp = 4; //TODO
        bpp * self.dim.w * self.dim.h * self.dim.d
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetView {
    inner: vk::ImageView,
    layer: u32,
    texture: Arc<Texture>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetSet {
    pub colors: Vec<(TargetView, Option<[f32; 4]>)>,
    pub depth_stencil: Option<(TargetView, Option<f32>, Option<u8>)>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineDesc {
    pub fun_vertex: Shader,
    pub fun_fragment: Shader,
}

pub struct PipelineLayout {
    inner: vk::PipelineLayout,
    _set_layouts: Vec<vk::DescriptorSetLayout>,
}

impl PipelineLayout {
    pub fn new(inner: vk::PipelineLayout,
               set_layouts: Vec<vk::DescriptorSetLayout>)
               -> PipelineLayout {
        PipelineLayout {
            inner: inner,
            _set_layouts: set_layouts,
        }
    }

    pub fn get_inner(&self) -> vk::PipelineLayout {
        self.inner
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pipeline {
    inner: vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl Pipeline {
    pub fn new(inner: vk::Pipeline, layout: vk::PipelineLayout) -> Pipeline {
        Pipeline {
            inner: inner,
            layout: layout,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ShaderType {
    Vertex,
    Fragment,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shader {
    inner: vk::ShaderModule,
}

impl Shader {
    pub fn new(inner: vk::ShaderModule) -> Shader {
        Shader {
            inner: inner,
        }
    }

    pub fn get_inner(&self) -> vk::ShaderModule {
        self.inner
    }
}

pub struct SwapChain {
    gpu_texture: Arc<Texture>,
    cpu_texture: Arc<Texture>,
    cpu_layer_count: u32,
    cpu_current_layer: u32,
    views: Vec<TargetView>,
}

impl SwapChain {
    pub fn get_targets(&self) -> Vec<TargetView> {
        self.views.clone()
    }

    pub fn get_dimensions(&self) -> Dimensions {
        self.gpu_texture.dim.clone()
    }

    pub fn fetch_frame(&mut self, share: &Share, res: &mut ResourceState,
                       com: &CommandBuffer, frame_index: u32) {
        self.cpu_current_layer += 1;
        if self.cpu_current_layer >= self.cpu_layer_count {
            self.cpu_current_layer = 0;
        }
        com.copy_texture(share, res, &self.gpu_texture, frame_index,
                         &self.cpu_texture, self.cpu_current_layer);
    }
}

pub struct Fence {
    inner: vk::Fence,
}

impl Fence {
    pub fn new(inner: vk::Fence) -> Fence {
        Fence {
            inner: inner,
        }
    }

    pub fn get_inner(&self) -> vk::Fence {
        self.inner
    }
}
