/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use euclid::Size2D;
use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;
use webgpu_component::gpu;

pub use webgpu_component::QueueType;


#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, HeapSizeOf)]
pub struct WebGpuContextId(pub usize);

pub type WebGpuSender<T> = ipc_channel::ipc::IpcSender<T>;
pub type WebGpuReceiver<T> = ipc_channel::ipc::IpcReceiver<T>;

pub fn webgpu_channel<T: Serialize + for<'de> Deserialize<'de>>(
) -> Result<(WebGpuSender<T>, WebGpuReceiver<T>), io::Error>
{
    ipc_channel::ipc::channel()
}

pub type AdapterId = u8;
pub type QueueFamilyId = u32;
pub type QueueCount = u8;
pub type QueueId = u32;
pub type DeviceId = u32;
pub type HeapId = u32;
pub type ImageId = u32;

/// Contains the WebGpuCommand sender and information about a WebGpuContext
#[derive(Clone, Deserialize, Serialize)]
pub struct ContextInfo {
    /// Sender instance to send commands to the specific WebGpuContext.
    pub sender: WebGpuMsgSender,
    /// Vector of available adapters.
    pub adapters: Vec<AdapterInfo>,
}

#[derive(Clone, Deserialize, Serialize, HeapSizeOf)]
pub struct QueueFamilyInfo {
    pub ty: QueueType,
    pub count: QueueCount,
    pub original_id: QueueFamilyId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AdapterInfo {
    pub info: gpu::AdapterInfo,
    pub queue_families: Vec<QueueFamilyInfo>,
    pub original_id: AdapterId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub general: Vec<QueueId>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SwapchainInfo {
    pub heap_id: HeapId,
    pub images: Vec<ImageId>,
}


/// WebGpu Command API
#[derive(Clone, Deserialize, Serialize)]
pub enum WebGpuCommand {
    Reset,
    Exit,
}

pub type WebGpuCommandChan = WebGpuSender<WebGpuCommand>;

#[derive(Clone, Deserialize, Serialize)]
pub struct CommandPoolInfo {
    pub channel: WebGpuCommandChan,
}


/// WebGpu Message API
#[derive(Clone, Deserialize, Serialize)]
pub enum WebGpuMsg {
    /// Creates a new WebGPU context instance.
    CreateContext(WebGpuSender<Result<ContextInfo, String>>),
    /// Create a new device on the adapter.
    OpenAdapter {
        adapter_id: AdapterId,
        queue_families: Vec<(QueueFamilyId, QueueCount)>,
        result: WebGpuSender<DeviceInfo>,
    },
    /// Build a new swapchain on the device.
    BuildSwapchain {
        device_id: DeviceId,
        size: Size2D<u32>,
        result: WebGpuSender<SwapchainInfo>,
    },
    CreateCommandPool {
        device_id: DeviceId,
        queue_id: QueueId,
        max_buffers: u32,
        result: WebGpuSender<CommandPoolInfo>,
    },
    /// Present the specified image on screen.
    Present(ImageId),
    /// Frees all resources and closes the thread.
    Exit,
}

pub type WebGpuChan = WebGpuSender<WebGpuMsg>;

#[derive(Clone, Deserialize, Serialize)]
pub struct WebGpuPipeline(pub WebGpuChan);

impl WebGpuPipeline {
    pub fn channel(&self) -> WebGpuChan {
        self.0.clone()
    }
}

/// Helper struct to send WebGpuCommands to a specific WebGpuContext.
#[derive(Clone, Deserialize, HeapSizeOf, Serialize)]
pub struct WebGpuMsgSender {
    ctx_id: WebGpuContextId,
    #[ignore_heap_size_of = "channels are hard"]
    pub sender: WebGpuChan,
}

impl WebGpuMsgSender {
    pub fn new(ctx_id: WebGpuContextId, sender: WebGpuChan) -> Self {
        WebGpuMsgSender {
            ctx_id,
            sender,
        }
    }
}
