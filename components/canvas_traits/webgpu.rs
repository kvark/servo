/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use euclid::Size2D;
use heapsize::HeapSizeOf;
use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;

pub use webgpu_component::gpu;


#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, HeapSizeOf)]
pub struct WebGpuContextId(pub usize);

pub type WebGpuSender<T> = ipc_channel::ipc::IpcSender<T>;
pub type WebGpuReceiver<T> = ipc_channel::ipc::IpcReceiver<T>;

pub fn webgpu_channel<T: Serialize + for<'de> Deserialize<'de>>(
) -> Result<(WebGpuSender<T>, WebGpuReceiver<T>), io::Error>
{
    ipc_channel::ipc::channel()
}

pub type Epoch = u32;
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, HeapSizeOf, Deserialize, Serialize)]
pub struct Key {
    pub index: usize, //TODO: u32
    pub epoch: u32,
}

pub type AdapterId = u8;
pub type GpuId = Key;
pub type QueueFamilyId = u32;
pub type QueueCount = u8;
pub type QueueId = u8;
pub type HeapId = Key;
pub type BufferId = Key;
pub type ImageId = Key;
pub type CommandBufferId = Key;
pub type CommandPoolId = Key;
pub type FenceId = Key;
pub type SemaphoreId = u32;
pub type SubmitEpoch = Epoch;
pub type FramebufferId = Key;
pub type RenderpassId = Key;
pub type RenderTargetViewId = Key;
pub type DepthStencilViewId = Key;

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
    pub ty: gpu::QueueType,
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
}

#[derive(Clone, Deserialize, Serialize, HeapSizeOf)]
pub struct SubmitInfo {
    pub pool_id: CommandPoolId,
    pub cb_id: CommandBufferId,
    pub submit_epoch: SubmitEpoch,
}

#[derive(Clone, Deserialize, Serialize, HeapSizeOf)]
pub struct RenderTargetViewInfo {
    pub id: RenderTargetViewId,
}

#[derive(Clone, Deserialize, Serialize, HeapSizeOf)]
pub struct DepthStencilViewInfo {
    pub id: DepthStencilViewId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct BufferBarrier {
    pub state_src: gpu::buffer::State,
    pub state_dst: gpu::buffer::State,
    pub target: BufferId,
}

impl HeapSizeOf for BufferBarrier {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ImageBarrier {
    pub state_src: gpu::image::State,
    pub state_dst: gpu::image::State,
    pub target: ImageId,
}

impl HeapSizeOf for ImageBarrier {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FramebufferInfo {
    pub id: FramebufferId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FramebufferDesc {
    pub renderpass: RenderpassId,
    pub colors: Vec<RenderTargetViewId>,
    pub depth_stencil: Option<DepthStencilViewId>,
    pub width: u32,
    pub height: u32,
    pub layers: u32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RenderpassInfo {
    pub id: RenderpassId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SubpassDesc {
    pub colors: Vec<gpu::pass::AttachmentRef>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RenderpassDesc {
    pub attachments: Vec<gpu::pass::Attachment>,
    pub subpasses: Vec<SubpassDesc>,
    pub dependencies: Vec<gpu::pass::SubpassDependency>,
}

/// WebGpu Command API
#[derive(Clone, Deserialize, Serialize)]
pub enum WebGpuCommand {
    Reset,
    Exit,
    AllocateCommandBuffers(u32, WebGpuSender<CommandBufferInfo>),
    FreeCommandBuffers(Vec<CommandBufferId>),
    Begin(CommandBufferId),
    Finish(SubmitEpoch),
    PipelineBarrier(Vec<BufferBarrier>, Vec<ImageBarrier>),
    BeginRenderpass {
        renderpass: RenderpassId,
        framebuffer: FramebufferId,
        area: gpu::target::Rect,
        clear_values: Vec<gpu::command::ClearValue>,
    },
    EndRenderpass,
}

pub type WebGpuCommandChan = WebGpuSender<WebGpuCommand>;

#[derive(Clone, Deserialize, Serialize)]
pub struct CommandPoolInfo {
    pub channel: WebGpuCommandChan,
    pub id: CommandPoolId,
}


/// WebGpu Message API
#[derive(Deserialize, Serialize)]
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
        format: gpu::format::Format,
        size: Size2D<u32>,
        result: WebGpuSender<SwapchainInfo>,
    },
    CreateCommandPool {
        gpu_id: GpuId,
        queue_id: QueueId,
        result: WebGpuSender<CommandPoolInfo>,
    },
    Submit {
        gpu_id: GpuId,
        queue_id: QueueId,
        command_buffers: Vec<SubmitInfo>,
        wait_semaphores: Vec<SemaphoreId>,
        signal_semaphores: Vec<SemaphoreId>,
        fence_id: Option<FenceId>,
    },
    CreateFence {
        gpu_id: GpuId,
        set: bool,
        result: WebGpuSender<FenceId>,
    },
    ResetFences {
        gpu_id: GpuId,
        fence_ids: Vec<FenceId>,
    },
    WaitForFences {
        gpu_id: GpuId,
        fence_ids: Vec<FenceId>,
        mode: gpu::device::WaitFor,
        timeout: u32,
        result: WebGpuSender<bool>,
    },
    CreateFramebuffer {
        gpu_id: GpuId,
        desc: FramebufferDesc,
        result: WebGpuSender<FramebufferInfo>,
    },
    CreateRenderpass {
        gpu_id: GpuId,
        desc: RenderpassDesc,
        result: WebGpuSender<RenderpassInfo>,
    },
    ViewImageAsRenderTarget {
        gpu_id: GpuId,
        image_id: ImageId,
        format: gpu::format::Format,
        result: WebGpuSender<RenderTargetViewInfo>,
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
