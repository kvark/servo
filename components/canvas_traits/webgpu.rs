/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use heapsize::HeapSizeOf;
use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;
use euclid::Size2D;
use webrender_api;

pub use webgpu_component::gpu;


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

pub type ContextId = Key;
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


#[derive(Clone, Copy, Deserialize, HeapSizeOf, Serialize)]
pub enum WebGpuContextShareMode {
    /// Fast: a shared texture_id is used in WebRender.
    SharedTexture,
    /// Slow: pixels are read back and sent to WebRender each frame.
    Readback,
}

/// Contains the WebGpuCommand sender and information about a WebGpuContext
#[derive(Clone, Deserialize, Serialize)]
pub struct ContextInfo {
    /// Produced context ID.
    pub id: ContextId,
    /// Vector of available adapters.
    pub adapters: Vec<AdapterInfo>,
    /// Sender instance to send commands to the GPU thread.
    pub sender: WebGpuChan,
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
    pub limits: gpu::Limits,
    pub general: Vec<QueueId>,
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

#[derive(Clone, Deserialize, Serialize)]
pub struct HeapDesc {
    pub size: usize,
    pub properties: gpu::memory::HeapProperties,
    pub resources: gpu::device::ResourceHeapType,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct HeapInfo {
    pub id: HeapId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct BufferDesc {
    pub size: usize,
    pub stride: usize,
    pub usage: gpu::buffer::Usage,
    pub heap_id: HeapId,
    pub heap_offset: usize,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct BufferInfo {
    pub id: BufferId,
    pub occupied_size: usize,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ImageDesc {
    pub kind: gpu::image::Kind,
    pub levels: gpu::image::Level,
    pub format: gpu::format::Format,
    pub usage: gpu::image::Usage,
    pub heap_id: HeapId,
    pub heap_offset: usize,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ImageInfo {
    pub id: ImageId,
    pub occupied_size: usize,
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
    CopyImageToBuffer {
        source_id: ImageId,
        source_layout: gpu::image::ImageLayout,
        destination_id: BufferId,
        regions: Vec<gpu::command::BufferImageCopy>,
    },
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
    CreateContext(Size2D<u32>, WebGpuSender<Result<ContextInfo, String>>),
    /// Create a new device on the adapter.
    OpenAdapter {
        adapter_id: AdapterId,
        queue_families: Vec<(QueueFamilyId, QueueCount)>,
        result: WebGpuSender<GpuInfo>,
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
    /// Present the specified image on screen.
    Present {
        context_id: ContextId,
        gpu_id: GpuId,
        buffer_id: BufferId,
        bytes_per_row: usize,
        fence_id: FenceId,
        size: Size2D<u32>
    },
    ReadWrImage(ContextId, WebGpuSender<webrender_api::ImageKey>),
    /// Frees all resources and closes the thread.
    Exit,
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
    CreateHeap {
        gpu_id: GpuId,
        desc: HeapDesc,
        result: WebGpuSender<HeapInfo>,
    },
    CreateBuffer {
        gpu_id: GpuId,
        desc: BufferDesc,
        result: WebGpuSender<BufferInfo>,
    },
    CreateImage {
        gpu_id: GpuId,
        desc: ImageDesc,
        result: WebGpuSender<ImageInfo>,
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
}

pub type WebGpuChan = WebGpuSender<WebGpuMsg>;

#[derive(Clone, Deserialize, Serialize)]
pub struct WebGpuPipeline(pub WebGpuChan);

impl WebGpuPipeline {
    pub fn channel(&self) -> WebGpuChan {
        self.0.clone()
    }
}
