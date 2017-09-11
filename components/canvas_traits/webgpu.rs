/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use heapsize::HeapSizeOf;
use ipc_channel;
use serde::{Deserialize, Serialize};
use std::io;
use std::ops::Range;
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
pub type ShaderModuleId = Key;


#[derive(Clone, Copy, Deserialize, HeapSizeOf, Serialize)]
pub enum WebGpuContextShareMode {
    /// Fast: a shared texture_id is used in WebRender.
    SharedTexture,
    /// Slow: pixels are read back and sent to WebRender each frame.
    Readback,
}

/// Contains the WebGpuCommand sender and information about a WebGpuContext
#[derive(Deserialize, Serialize)]
pub struct ContextInfo {
    /// Presenter channel for showing the frames.
    pub presenter: Presenter,
    /// Vector of available adapters.
    pub adapters: Vec<AdapterInfo>,
    /// Sender instance to send commands to the GPU thread.
    pub sender: WebGpuChan,
    /// An image key for the currently presenting frame.
    pub image_key: webrender_api::ImageKey,
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
    pub states: Range<gpu::buffer::State>,
    pub target: BufferId,
}

impl HeapSizeOf for BufferBarrier {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ImageBarrier {
    pub states: Range<gpu::image::State>,
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
    pub extent: gpu::device::Extent,
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

#[derive(Clone, Deserialize, Serialize)]
pub struct ShaderModuleInfo {
    pub id: ShaderModuleId,
}


pub type PresentDone = bool;

#[derive(Deserialize, Serialize, HeapSizeOf)]
pub struct ReadyFrame {
    pub gpu_id: GpuId,
    pub buffer_id: BufferId,
    pub bytes_per_row: usize,
    pub fence_id: FenceId,
    pub size: Size2D<u32>,
    #[ignore_heap_size_of = "Channels are hard"]
    pub done_event: Option<WebGpuSender<PresentDone>>,
}

impl ReadyFrame {
    pub fn reuse(mut self) -> Self {
        //println!("frame with buffer id {:?} is reused", self.buffer_id);
        self.done_event.take().unwrap().send(true).unwrap();
        self
    }
    pub fn consume(mut self, shown: bool) {
        //println!("frame with buffer id {:?} is consumed with result: {}", self.buffer_id, shown);
        if let Some(sender) = self.done_event.take() {
            sender.send(shown).unwrap();
        }
    }
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
    PipelineBarrier {
        stages: Range<gpu::pso::PipelineStage>,
        buffer_bars: Vec<BufferBarrier>,
        image_bars: Vec<ImageBarrier>,
    },
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
    CreateContext {
        size: Size2D<u32>,
        external_image_id: webrender_api::ExternalImageId,
        result: WebGpuSender<Result<ContextInfo, String>>,
    },
    /// Create a new device on the adapter.
    OpenAdapter {
        adapter_id: AdapterId,
        queue_families: Vec<(QueueFamilyId, QueueCount)>,
        result: WebGpuSender<GpuInfo>,
    },
    CreateCommandPool {
        gpu_id: GpuId,
        queue_id: QueueId,
        flags: gpu::pool::CommandPoolCreateFlags,
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
    Present {
        image_key: webrender_api::ImageKey,
        external_image_id: webrender_api::ExternalImageId,
        size: Size2D<u32>,
        stride: u32,
    },
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
    CreateShaderModule {
        gpu_id: GpuId,
        data: Vec<u8>,
        result: WebGpuSender<ShaderModuleInfo>,
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

/// WebGpu presenter command type
#[derive(Deserialize, Serialize)]
pub enum WebGpuPresent {
    Enter,
    Exit,
    Show(ReadyFrame),
}

pub type WebGpuPresentChan = WebGpuSender<(webrender_api::ExternalImageId, WebGpuPresent)>;

#[derive(Clone, Deserialize, Serialize)]
pub struct Presenter {
    pub id: webrender_api::ExternalImageId,
    pub channel: WebGpuPresentChan,
}

impl HeapSizeOf for Presenter {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

impl Presenter {
    pub fn send(&self, present: WebGpuPresent) {
        self.channel.send((self.id, present)).unwrap()
    }
}
