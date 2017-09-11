/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::sync::{Arc, Mutex,RwLock};
use webgpu::gpu;
use super::LazyVec;

pub struct ResourceHub<B: gpu::Backend> {
    pub gpus: Mutex<LazyVec<gpu::Gpu<B>>>,
    pub buffers: RwLock<LazyVec<B::Buffer>>,
    pub images: RwLock<LazyVec<B::Image>>,
    pub framebuffers: RwLock<LazyVec<B::FrameBuffer>>,
    pub renderpasses: RwLock<LazyVec<B::RenderPass>>,
    pub rtvs: RwLock<LazyVec<B::RenderTargetView>>,
    pub dsvs: RwLock<LazyVec<B::DepthStencilView>>,
    pub fences: RwLock<LazyVec<B::Fence>>,
    pub shaders: RwLock<LazyVec<B::ShaderModule>>,
}

impl<B: gpu::Backend> ResourceHub<B> {
    pub fn new() -> Arc<Self> {
        Arc::new(ResourceHub {
            gpus: Mutex::new(LazyVec::new()),
            buffers: RwLock::new(LazyVec::new()),
            images: RwLock::new(LazyVec::new()),
            framebuffers: RwLock::new(LazyVec::new()),
            renderpasses: RwLock::new(LazyVec::new()),
            rtvs: RwLock::new(LazyVec::new()),
            dsvs: RwLock::new(LazyVec::new()),
            fences: RwLock::new(LazyVec::new()),
            shaders: RwLock::new(LazyVec::new()),
        })
    }
}
