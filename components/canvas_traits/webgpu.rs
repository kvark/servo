/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use hal;
use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;
//use std::ops::Range;
use euclid::Size2D;
use malloc_size_of::{MallocSizeOf, MallocSizeOfOps};
use webrender_api;


pub type WebGPUSender<T> = ipc_channel::ipc::IpcSender<T>;
pub type WebGPUReceiver<T> = ipc_channel::ipc::IpcReceiver<T>;

pub fn webgpu_channel<T: Serialize + for<'de> Deserialize<'de>>(
) -> Result<(WebGPUSender<T>, WebGPUReceiver<T>), io::Error>
{
    ipc_channel::ipc::channel()
}

pub type Epoch = u32;
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, MallocSizeOf, Deserialize, Serialize)]
pub struct Key {
    pub index: u32,
    pub epoch: u32,
}

pub type SwapchainId = Key;
pub type DeviceId = Key;

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceInfo {
    /// Selected adapter info.
    pub adapter_info: hal::AdapterInfo,
    /// Supported features.
    pub features: hal::Features,
    /// Supported limits.
    pub limits: hal::Limits,
}

impl MallocSizeOf for InstanceInfo {
    fn size_of(&self, ops: &mut MallocSizeOfOps) -> usize {
        self.adapter_info.name.size_of(ops)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContextInfo {
    /// Instance info.
    pub info: InstanceInfo,
    /// Swapchain identifier.
    pub swapchain: SwapchainId,
    /// An image key for the currently presenting frame.
    pub image_key: webrender_api::ImageKey,
}


/// WebGPU Message API
#[derive(Debug, Deserialize, Serialize)]
pub enum WebGPUMsg {
    /// Creates a new WebGPU context instance.
    CreateContext {
        size: Size2D<u32>,
        //external_image_id: webrender_api::ExternalImageId,
        result: WebGPUSender<Result<ContextInfo, String>>,
    },
    Exit,
}

pub type WebGPUMainChan = WebGPUSender<WebGPUMsg>;

#[derive(Clone, Deserialize, Serialize)]
pub struct WebGPUPipeline(WebGPUMainChan);

impl WebGPUPipeline {
    pub fn new(chan: WebGPUMainChan) -> Self {
        WebGPUPipeline(chan)
    }

    pub fn channel(&self) -> WebGPUMainChan {
        self.0.clone()
    }
}
