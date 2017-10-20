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

pub use webgpu_component::hal;

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
pub type MemoryId = Key;
pub type BufferId = Key;
pub type ImageId = Key;
pub type ImageViewId = Key;
pub type CommandBufferId = Key;
pub type CommandPoolId = Key;
pub type FenceId = Key;
pub type SemaphoreId = u32;
pub type SubmitEpoch = Epoch;
pub type FramebufferId = Key;
pub type RenderPassId = Key;
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
#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueueFamilyInfo {
    pub ty: hal::QueueType,
    pub count: QueueCount,
    pub original_id: QueueFamilyId,
}

impl HeapSizeOf for QueueFamilyInfo {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdapterInfo {
    pub info: hal::AdapterInfo,
    pub queue_families: Vec<QueueFamilyInfo>,
    pub original_id: AdapterId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GpuInfo {
    pub id: GpuId,
    pub limits: hal::Limits,
    pub mem_types: Vec<hal::MemoryType>,
    pub general: Vec<QueueId>,
}

#[derive(Clone, Debug, Deserialize, Serialize, HeapSizeOf)]
pub struct CommandBufferInfo {
    pub id: CommandBufferId,
}

#[derive(Clone, Debug, Deserialize, Serialize, HeapSizeOf)]
pub struct SubmitInfo {
    pub pool_id: CommandPoolId,
    pub cb_id: CommandBufferId,
    pub submit_epoch: SubmitEpoch,
}

#[derive(Clone, Debug, Deserialize, Serialize, HeapSizeOf)]
pub struct ImageViewInfo {
    pub id: ImageViewId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BufferBarrier {
    pub states: Range<hal::buffer::State>,
    pub target: BufferId,
}

impl HeapSizeOf for BufferBarrier {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImageBarrier {
    pub states: Range<hal::image::State>,
    pub target: ImageId,
}

impl HeapSizeOf for ImageBarrier {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FramebufferInfo {
    pub id: FramebufferId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FramebufferDesc {
    pub render_pass: RenderPassId,
    pub attachments: Vec<ImageViewId>,
    pub extent: hal::device::Extent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderPassInfo {
    pub id: RenderPassId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubpassDesc {
    pub colors: Vec<hal::pass::AttachmentRef>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderPassDesc {
    pub attachments: Vec<hal::pass::Attachment>,
    pub subpasses: Vec<SubpassDesc>,
    pub dependencies: Vec<hal::pass::SubpassDependency>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryDesc {
    pub size: usize,
    pub ty: hal::MemoryType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryInfo {
    pub id: MemoryId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BufferDesc {
    pub size: usize,
    pub stride: usize,
    pub usage: hal::buffer::Usage,
    pub mem_id: MemoryId,
    pub mem_offset: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BufferInfo {
    pub id: BufferId,
    pub occupied_size: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImageDesc {
    pub kind: hal::image::Kind,
    pub levels: hal::image::Level,
    pub format: hal::format::Format,
    pub usage: hal::image::Usage,
    pub mem_id: MemoryId,
    pub mem_offset: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImageInfo {
    pub id: ImageId,
    pub occupied_size: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShaderModuleInfo {
    pub id: ShaderModuleId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DescriptorSetLayoutInfo {
    pub id: DescriptorSetLayoutId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineLayoutInfo {
    pub id: PipelineLayoutId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DescriptorPoolInfo {
    pub id: DescriptorPoolId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DescriptorSetInfo {
    pub id: DescriptorSetId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GraphicsPipelineInfo {
    pub id: GraphicsPipelineId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SamplerInfo {
    pub id: SamplerId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntryPoint {
    pub module_id: ShaderModuleId,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GraphicsShaderSet {
    pub vs: EntryPoint,
    pub fs: Option<EntryPoint>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GraphicsPipelineDesc {
    pub shaders: GraphicsShaderSet,
    pub layout_id: PipelineLayoutId,
    pub renderpass_id: RenderPassId,
    pub subpass: u32,
    pub inner: hal::pso::GraphicsPipelineDesc,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DescriptorSetWrite {
    pub set: DescriptorSetId,
    pub binding: usize,
    pub array_offset: usize,
    pub ty: hal::pso::DescriptorType,
    pub descriptors: Vec<(Key, hal::image::ImageLayout)>, //careful!
}


pub type PresentDone = bool;
pub type SubmitStart = ();

#[derive(Debug, Deserialize, Serialize, HeapSizeOf)]
pub struct ReadyFrame {
    pub buffer_id: BufferId,
    pub bytes_per_row: usize,
    pub fence_id: FenceId,
    pub size: Size2D<u32>,
    #[ignore_heap_size_of = "Channels are hard"]
    pub done_event: Option<WebGpuSender<PresentDone>>,
    #[ignore_heap_size_of = "Channels are hard"]
    pub wait_event: Option<WebGpuReceiver<SubmitStart>>,
}

impl ReadyFrame {
    pub fn reuse(mut self) -> Self {
        //println!("frame with buffer id {:?} is reused", self.buffer_id);
        if let Some(sender) = self.done_event.take() {
            sender.send(true).unwrap();
        }
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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum WebGpuCommand {
    Reset,
    Exit,
    AllocateCommandBuffers(u32, WebGpuSender<CommandBufferInfo>),
    FreeCommandBuffers(Vec<CommandBufferId>),
    Begin(CommandBufferId),
    Finish(SubmitEpoch),
    PipelineBarrier {
        stages: Range<hal::pso::PipelineStage>,
        buffer_bars: Vec<BufferBarrier>,
        image_bars: Vec<ImageBarrier>,
    },
    BeginRenderPass {
        render_pass: RenderPassId,
        framebuffer: FramebufferId,
        area: hal::target::Rect,
        clear_values: Vec<hal::command::ClearValue>,
    },
    EndRenderPass,
    CopyBufferToImage {
        source_id: BufferId,
        dest_id: ImageId,
        dest_layout: hal::image::ImageLayout,
        regions: Vec<hal::command::BufferImageCopy>,
    },
    CopyImageToBuffer {
        source_id: ImageId,
        source_layout: hal::image::ImageLayout,
        dest_id: BufferId,
        regions: Vec<hal::command::BufferImageCopy>,
    },
    BindGraphicsPipeline(GraphicsPipelineId),
    BindGraphicsDescriptorSets {
        layout_id: PipelineLayoutId,
        desc_offset: usize,
        set_ids: Vec<DescriptorSetId>,
    },
    SetScissors(Vec<hal::target::Rect>),
    SetViewports(Vec<hal::Viewport>),
    Draw(Range<hal::VertexCount>, Range<hal::InstanceCount>),
}

pub type WebGpuCommandChan = WebGpuSender<WebGpuCommand>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandPoolInfo {
    pub channel: WebGpuCommandChan,
    pub id: CommandPoolId,
}


/// WebGpu Message API
#[derive(Debug, Deserialize, Serialize)]
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
        flags: hal::pool::CommandPoolCreateFlags,
        result: WebGpuSender<CommandPoolInfo>,
    },
    Submit {
        gpu_id: GpuId,
        queue_id: QueueId,
        command_buffers: Vec<SubmitInfo>,
        wait_semaphores: Vec<SemaphoreId>,
        signal_semaphores: Vec<SemaphoreId>,
        fence_id: Option<FenceId>,
        feedback: Option<WebGpuSender<SubmitStart>>,
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
        mode: hal::device::WaitFor,
        timeout: u32,
        result: WebGpuSender<bool>,
    },
    AllocateMemory {
        gpu_id: GpuId,
        desc: MemoryDesc,
        result: WebGpuSender<MemoryInfo>,
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
    CreateImageView {
        gpu_id: GpuId,
        image_id: ImageId,
        format: hal::format::Format,
        range: hal::image::SubresourceRange,
        result: WebGpuSender<ImageViewInfo>,
    },
    CreateFramebuffer {
        gpu_id: GpuId,
        desc: FramebufferDesc,
        result: WebGpuSender<FramebufferInfo>,
    },
    CreateRenderPass {
        gpu_id: GpuId,
        desc: RenderPassDesc,
        result: WebGpuSender<RenderPassInfo>,
    },
    CreateDescriptorSetLayout {
        gpu_id: GpuId,
        bindings: Vec<hal::pso::DescriptorSetLayoutBinding>,
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
        ranges: Vec<hal::pso::DescriptorRangeDesc>,
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
    #[cfg(windows)]
    CreateShaderModuleHLSL {
        gpu_id: GpuId,
        stage: hal::pso::Stage,
        data: Vec<u8>,
        result: WebGpuSender<ShaderModuleInfo>,
    },
    #[cfg(target_os = "macos")]
    CreateShaderModuleMSL {
        gpu_id: GpuId,
        data: String,
        result: WebGpuSender<ShaderModuleInfo>,
    },
    CreateGraphicsPipelines {
        gpu_id: GpuId,
        descriptors: Vec<GraphicsPipelineDesc>,
        result: WebGpuSender<GraphicsPipelineInfo>,
    },
    CreateSampler {
        gpu_id: GpuId,
        desc: hal::image::SamplerInfo,
        result: WebGpuSender<SamplerInfo>,
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
#[derive(Debug, Deserialize, Serialize)]
pub enum WebGpuPresent {
    Enter(GpuId),
    Exit,
    Show(ReadyFrame),
}

pub type WebGpuPresentChan = WebGpuSender<(webrender_api::ExternalImageId, WebGpuPresent)>;

#[derive(Clone, Debug, Deserialize, Serialize)]
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
