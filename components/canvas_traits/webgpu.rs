/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use euclid::Size2D;
use heapsize::HeapSizeOf;
use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;
use webgpu_component::gpu;


pub use webgpu_component::gpu::QueueType;
pub use webgpu_component::gpu::buffer::State as BufferState;
pub use webgpu_component::gpu::image::State as ImageState;

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
pub type GpuId = u32;
pub type QueueFamilyId = u32;
pub type QueueCount = u8;
pub type QueueId = u32;
pub type HeapId = u32;
pub type BufferId = u32;
pub type ImageId = u32;
pub type CommandBufferId = u32;
pub type CommandBufferEpoch = u32;
pub type CommandPoolId = u64;
pub type FenceId = u32;
pub type SemaphoreId = u32;
pub type SubmitEpoch = u32;

/// Contains the WebGpuCommand sender and information about a WebGpuContext
#[derive(Clone, Deserialize, Serialize)]
pub struct ContextInfo {
    /// Sender instance to send commands to the specific WebGpuContext.
    pub sender: WebGpuMsgSender,
    /// Vector of available adapters.
    pub adapters: Vec<AdapterInfo>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct QueueFamilyInfo {
    pub ty: QueueType,
    pub count: QueueCount,
    pub original_id: QueueFamilyId,
}

impl HeapSizeOf for QueueFamilyInfo {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AdapterInfo {
    pub info: gpu::AdapterInfo,
    pub queue_families: Vec<QueueFamilyInfo>,
    pub original_id: AdapterId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GpuInfo {
    pub id: GpuId,
    pub general: Vec<QueueId>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SwapchainInfo {
    pub heap_id: HeapId,
    pub images: Vec<ImageId>,
}

#[derive(Clone, Deserialize, Serialize, HeapSizeOf)]
pub struct CommandBufferInfo {
    pub id: CommandBufferId,
    pub epoch: CommandBufferEpoch,
}

#[derive(Clone, Deserialize, Serialize, HeapSizeOf)]
pub struct SubmitInfo {
    pub pool_id: CommandPoolId,
    pub cb: CommandBufferInfo,
    pub submit_epoch: SubmitEpoch,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct BufferBarrier {
    pub state_src: BufferState,
    pub state_dst: BufferState,
    pub target: BufferId,
}

impl HeapSizeOf for BufferBarrier {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ImageBarrier {
    pub state_src: ImageState,
    pub state_dst: ImageState,
    pub target: ImageId,
}

impl HeapSizeOf for ImageBarrier {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

/// WebGpu Command API
#[derive(Clone, Deserialize, Serialize)]
pub enum WebGpuCommand {
    Reset,
    Exit,
    AcquireCommandBuffer(WebGpuSender<CommandBufferInfo>),
    ReturnCommandBuffer(CommandBufferId),
    Finish(CommandBufferInfo, SubmitEpoch),
    PipelineBarrier(Vec<BufferBarrier>, Vec<ImageBarrier>),
}

pub type WebGpuCommandChan = WebGpuSender<WebGpuCommand>;

#[derive(Clone, Deserialize, Serialize)]
pub struct CommandPoolInfo {
    pub channel: WebGpuCommandChan,
    pub id: CommandPoolId,
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
        result: WebGpuSender<GpuInfo>,
    },
    /// Build a new swapchain on the device.
    BuildSwapchain {
        gpu_id: GpuId,
        size: Size2D<u32>,
        result: WebGpuSender<SwapchainInfo>,
    },
    CreateCommandPool {
        gpu_id: GpuId,
        queue_id: QueueId,
        max_buffers: u32,
        result: WebGpuSender<CommandPoolInfo>,
    },
    Submit {
        gpu_id: GpuId,
        queue_id: QueueId,
        command_buffers: Vec<SubmitInfo>,
        wait_semaphores: Vec<SemaphoreId>,
        signal_semaphores: Vec<SemaphoreId>,
        fence: Option<FenceId>,
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
