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
pub type ShaderResourceViewId = Key;
pub type DescriptorSetLayoutId = Key;
pub type PipelineLayoutId = Key;
pub type DescriptorPoolId = Key;
pub type DescriptorSetId = Key;
pub type ShaderModuleId = Key;
pub type GraphicsPipelineId = Key;
pub type SamplerId = Key;


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
    pub heap_types: Vec<gpu::HeapType>,
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

#[derive(Clone, Deserialize, Serialize, HeapSizeOf)]
pub struct ShaderResourceViewInfo {
    pub id: ShaderResourceViewId,
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
    pub ty: gpu::HeapType,
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

#[derive(Clone, Deserialize, Serialize)]
pub struct DescriptorSetLayoutInfo {
    pub id: DescriptorSetLayoutId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PipelineLayoutInfo {
    pub id: PipelineLayoutId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DescriptorPoolInfo {
    pub id: DescriptorPoolId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DescriptorSetInfo {
    pub id: DescriptorSetId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GraphicsPipelineInfo {
    pub id: GraphicsPipelineId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SamplerInfo {
    pub id: SamplerId,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct EntryPoint {
    pub module_id: ShaderModuleId,
    pub name: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GraphicsShaderSet {
    pub vs: EntryPoint,
    pub fs: Option<EntryPoint>,
}

#[derive(Deserialize, Serialize)]
pub struct GraphicsPipelineDesc {
    pub shaders: GraphicsShaderSet,
    pub layout_id: PipelineLayoutId,
    pub renderpass_id: RenderpassId,
    pub subpass: u32,
    pub inner: gpu::pso::GraphicsPipelineDesc,
}

#[derive(Deserialize, Serialize)]
pub struct DescriptorSetWrite {
    pub set: DescriptorSetId,
    pub binding: usize,
    pub array_offset: usize,
    pub ty: gpu::pso::DescriptorType,
    pub descriptors: Vec<(Key, gpu::image::ImageLayout)>, //careful!
}


pub type PresentDone = bool;

#[derive(Deserialize, Serialize, HeapSizeOf)]
pub struct ReadyFrame {
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
    CopyBufferToImage {
        source_id: BufferId,
        dest_id: ImageId,
        dest_layout: gpu::image::ImageLayout,
        regions: Vec<gpu::command::BufferImageCopy>,
    },
    CopyImageToBuffer {
        source_id: ImageId,
        source_layout: gpu::image::ImageLayout,
        dest_id: BufferId,
        regions: Vec<gpu::command::BufferImageCopy>,
    },
    BindGraphicsPipeline(GraphicsPipelineId),
    SetScissors(Vec<gpu::target::Rect>),
    SetViewports(Vec<gpu::Viewport>),
    Draw(Range<gpu::VertexCount>, Range<gpu::InstanceCount>),
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
    CreateDescriptorSetLayout {
        gpu_id: GpuId,
        bindings: Vec<gpu::pso::DescriptorSetLayoutBinding>,
        result: WebGpuSender<DescriptorSetLayoutInfo>,
    },
    CreatePipelineLayout {
        gpu_id: GpuId,
        set_layout_ids: Vec<DescriptorSetLayoutId>,
        result: WebGpuSender<PipelineLayoutInfo>,
    },
    CreateDescriptorPool {
        gpu_id: GpuId,
        max_sets: usize,
        ranges: Vec<gpu::pso::DescriptorRangeDesc>,
        result: WebGpuSender<DescriptorPoolInfo>,
    },
    AllocateDescriptorSets {
        pool_id: DescriptorPoolId,
        set_layout_ids: Vec<DescriptorSetLayoutId>,
        result: WebGpuSender<DescriptorSetInfo>,
    },
    CreateShaderModule {
        gpu_id: GpuId,
        data: Vec<u8>,
        result: WebGpuSender<ShaderModuleInfo>,
    },
    CreateGraphicsPipelines {
        gpu_id: GpuId,
        descriptors: Vec<GraphicsPipelineDesc>,
        result: WebGpuSender<GraphicsPipelineInfo>,
    },
    CreateSampler {
        gpu_id: GpuId,
        desc: gpu::image::SamplerInfo,
        result: WebGpuSender<SamplerInfo>,
    },
    ViewImageAsRenderTarget {
        gpu_id: GpuId,
        image_id: ImageId,
        format: gpu::format::Format,
        result: WebGpuSender<RenderTargetViewInfo>,
    },
    ViewImageAsShaderResource {
        gpu_id: GpuId,
        image_id: ImageId,
        format: gpu::format::Format,
        result: WebGpuSender<ShaderResourceViewInfo>,
    },
    UploadBufferData {
        gpu_id: GpuId,
        buffer_id: BufferId,
        data: Vec<u8>, //TODO?
    },
    UpdateDescriptorSets {
        gpu_id: GpuId,
        writes: Vec<DescriptorSetWrite>,
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
    Enter(GpuId),
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
