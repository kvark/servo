/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::sync::{Arc, Mutex, RwLock};
use canvas_traits::hal;
use super::LazyVec;


pub struct ResourceHub<B: hal::Backend> {
    //TODO: consider moving to `WebGPUThread`
    pub buffers: RwLock<LazyVec<B::Buffer>>,
    pub images: RwLock<LazyVec<B::Image>>,
    pub image_views: RwLock<LazyVec<B::ImageView>>,
    pub framebuffers: RwLock<LazyVec<B::Framebuffer>>,
    pub render_passes: RwLock<LazyVec<B::RenderPass>>,
    pub fences: RwLock<LazyVec<B::Fence>>,
    pub shaders: RwLock<LazyVec<B::ShaderModule>>,
    pub set_layouts: RwLock<LazyVec<B::DescriptorSetLayout>>,
    pub pipe_layouts: RwLock<LazyVec<B::PipelineLayout>>,
    pub pools: Mutex<LazyVec<B::DescriptorPool>>,
    pub descriptors: RwLock<LazyVec<B::DescriptorSet>>,
    pub graphics_pipes: RwLock<LazyVec<B::GraphicsPipeline>>,
    pub samplers: RwLock<LazyVec<B::Sampler>>,
}

impl<B: hal::Backend> ResourceHub<B> {
    pub fn new() -> Arc<Self> {
        Arc::new(ResourceHub {
            buffers: RwLock::new(LazyVec::new()),
            images: RwLock::new(LazyVec::new()),
            image_views: RwLock::new(LazyVec::new()),
            framebuffers: RwLock::new(LazyVec::new()),
            render_passes: RwLock::new(LazyVec::new()),
            fences: RwLock::new(LazyVec::new()),
            shaders: RwLock::new(LazyVec::new()),
            set_layouts: RwLock::new(LazyVec::new()),
            pipe_layouts: RwLock::new(LazyVec::new()),
            pools: Mutex::new(LazyVec::new()),
            descriptors: RwLock::new(LazyVec::new()),
            graphics_pipes: RwLock::new(LazyVec::new()),
            samplers: RwLock::new(LazyVec::new()),
        })
    }
}
