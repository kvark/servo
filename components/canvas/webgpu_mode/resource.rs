/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::sync::{Arc, RwLock};
use webgpu::gpu;
use super::LazyVec;

pub struct ResourceHub<B: gpu::Backend> {
    pub heaps: RwLock<LazyVec<B::Heap>>,
    pub buffers: RwLock<LazyVec<B::Buffer>>,
    pub images: RwLock<LazyVec<B::Image>>,
}

impl<B: gpu::Backend> ResourceHub<B> {
    pub fn new() -> Arc<Self> {
        Arc::new(ResourceHub {
            heaps: RwLock::new(LazyVec::new()),
            buffers: RwLock::new(LazyVec::new()),
            images: RwLock::new(LazyVec::new()),
        })
    }
}
