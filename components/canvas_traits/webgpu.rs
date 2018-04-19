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

pub type SwapChainId = Key;
pub type DeviceId = Key;
pub type QueueId = Key;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, MallocSizeOf, Deserialize, Serialize)]
pub enum TextureId {
    Owned(Key),
    Swapchain(SwapChainId, usize),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
pub struct DeviceInfo {
    pub id: DeviceId,
    pub queue_id: QueueId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SwapChainInfo {
    /// Unique identifier.
    pub id: SwapChainId,
    /// A texture per swapchain image.
    pub textures: Vec<TextureInfo>,
    /// An image key for the currently presenting frame.
    pub image_key: webrender_api::ImageKey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextureInfo {
    /// Unique identifier.
    pub id: TextureId,
}


/// WebGPU Message API
#[derive(Debug, Deserialize, Serialize)]
pub enum Message {
    /// Initializes a new WebGPU instance.
    Init {
        result: WebGPUSender<Result<InstanceInfo, String>>,
    },
    /// Ceases all operations and exit.
    Exit,
    /// Creates a new logical device.
    CreateDevice {
        result: WebGPUSender<Result<DeviceInfo, String>>,
    },
    /// Creates a new WebGPU swap chain.
    CreateSwapChain {
        device: DeviceId,
        //queue: QueueId,
        size: Size2D<u32>,
        //external_image_id: webrender_api::ExternalImageId,
        result: WebGPUSender<Result<SwapChainInfo, String>>,
    },
    /// Gets the next available frame from the swapchain.
    AcquireFrame {
        device: DeviceId,
        swapchain: SwapChainId,
        result: WebGPUSender<TextureInfo>,
    },
    /// Presents a swapchain frame.
    Present {
        queue: QueueId,
        swapchain: SwapChainId,
    },
}

pub type WebGPUMainChan = WebGPUSender<Message>;

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
