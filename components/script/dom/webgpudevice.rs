/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{self as w, hal, webgpu_channel,
    WebGpuChan, WebGpuMsg};
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::globalscope::GlobalScope;
use dom::webgpubuffer::WebGpuBuffer;
use dom::webgpudescriptorpool::WebGpuDescriptorPool;
use dom::webgpudescriptorsetlayout::WebGpuDescriptorSetLayout;
use dom::webgpufence::WebGpuFence;
use dom::webgpuframebuffer::WebGpuFramebuffer;
use dom::webgpugraphicspipeline::WebGpuGraphicsPipeline;
use dom::webgpuheap::WebGpuHeap;
use dom::webgpuimage::WebGpuImage;
use dom::webgpuimageview::WebGpuImageView;
use dom::webgpupipelinelayout::WebGpuPipelineLayout;
use dom::webgpurenderpass::WebGpuRenderPass;
use dom::webgpusampler::WebGpuSampler;
use dom::webgpushadermodule::WebGpuShaderModule;
use dom_struct::dom_struct;
use heapsize::HeapSizeOf;
use js::jsapi::{JSContext, JSObject};
use glsl_to_spirv;


pub struct LimitsWrapper(pub hal::Limits);
impl HeapSizeOf for LimitsWrapper {
    fn heap_size_of_children(&self) -> usize { 0 }
}
pub struct MemTypeWrapper(pub hal::MemoryType);
impl HeapSizeOf for MemTypeWrapper {
    fn heap_size_of_children(&self) -> usize { 0 }
}


#[dom_struct]
pub struct WebGpuDevice {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    id: w::GpuId,
    limits: LimitsWrapper,
    mem_types: Vec<MemTypeWrapper>,
}

impl WebGpuDevice {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        id: w::GpuId,
        limits: hal::Limits,
        mem_types: &[hal::MemoryType],
    ) -> Root<Self>
    {
        let obj = box WebGpuDevice {
            reflector_: Reflector::new(),
            sender,
            id,
            limits: LimitsWrapper(limits),
            mem_types: mem_types
                .iter()
                .cloned()
                .map(MemTypeWrapper)
                .collect(),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn map_image_layout(layout: binding::WebGpuImageLayout) -> hal::image::ImageLayout {
        map_enum!(layout; self::binding::WebGpuImageLayout => self::hal::image::ImageLayout {
            General, ColorAttachmentOptimal, DepthStencilAttachmentOptimal,
            DepthStencilReadOnlyOptimal, ShaderReadOnlyOptimal,
            TransferSrcOptimal, TransferDstOptimal,
            Undefined, Preinitialized, Present
        })
    }

    pub fn map_format(format: binding::WebGpuFormat) -> hal::format::Format {
        use self::binding::WebGpuFormat::*;
        use self::hal::format::{Format, SurfaceType, ChannelType};
        match format {
            R8G8B8A8_UNORM => Format(SurfaceType::R8_G8_B8_A8, ChannelType::Unorm),
            R8G8B8A8_SRGB => Format(SurfaceType::R8_G8_B8_A8, ChannelType::Srgb),
            B8G8R8A8_UNORM => Format(SurfaceType::B8_G8_R8_A8, ChannelType::Unorm),
            B8G8R8A8_SRGB => Format(SurfaceType::B8_G8_R8_A8, ChannelType::Srgb),
        }
    }

    fn map_load_op(op: binding::WebGpuAttachmentLoadOp) -> hal::pass::AttachmentLoadOp {
        use self::binding::WebGpuAttachmentLoadOp::*;
        use self::hal::pass::AttachmentLoadOp as Alo;
        match op {
            Load => Alo::Load,
            Clear => Alo::Clear,
            DontCare => Alo::DontCare,
        }
    }

    fn map_store_op(op: binding::WebGpuAttachmentStoreOp) -> hal::pass::AttachmentStoreOp {
        use self::binding::WebGpuAttachmentStoreOp::*;
        use self::hal::pass::AttachmentStoreOp as Aso;
        match op {
            Store => Aso::Store,
            DontCare => Aso::DontCare,
        }
    }

    fn map_pass_ref(pass_ref: Option<u32>) -> hal::pass::SubpassRef {
        match pass_ref {
            Some(id) => hal::pass::SubpassRef::Pass(id as _),
            None => hal::pass::SubpassRef::External,
        }
    }

    fn map_descriptor_type(ty: binding::WebGpuDescriptorType) -> hal::pso::DescriptorType {
        map_enum!(ty; self::binding::WebGpuDescriptorType => self::hal::pso::DescriptorType {
            Sampler, SampledImage, StorageImage, UniformTexelBuffer, StorageTexelBuffer,
            UniformBuffer, StorageBuffer, InputAttachment
        })
    }
}

impl binding::WebGpuDeviceMethods for WebGpuDevice {
    fn GetLimits(&self) -> binding::WebGpuDeviceLimits {
        binding::WebGpuDeviceLimits {
            minBufferCopyOffsetAlignment: self.limits.0.min_buffer_copy_offset_alignment as _,
            minBufferCopyPitchAlignment: self.limits.0.min_buffer_copy_pitch_alignment as _,
        }
    }

    fn CreateFence(&self, set: bool) -> Root<WebGpuFence> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateFence {
            gpu_id: self.id,
            set,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let fence = receiver.recv().unwrap();
        WebGpuFence::new(&self.global(), fence)
    }

    fn ResetFences(&self, fences: Vec<Root<WebGpuFence>>) {
        let fence_ids = fences
            .into_iter()
            .map(|f| f.get_id())
            .collect();

        let msg = WebGpuMsg::ResetFences {
            gpu_id: self.id,
            fence_ids,
        };
        self.sender.send(msg).unwrap();
    }

    fn WaitForFences(
        &self,
        fences: Vec<Root<WebGpuFence>>,
        wait_mode: binding::WebGpuFenceWait,
        timeout: u32,
    ) -> bool {
        let fence_ids = fences
            .into_iter()
            .map(|f| f.get_id())
            .collect();
        let mode = match wait_mode {
            binding::WebGpuFenceWait::Any => hal::device::WaitFor::Any,
            binding::WebGpuFenceWait::All => hal::device::WaitFor::All,
        };

        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::WaitForFences {
            gpu_id: self.id,
            fence_ids,
            mode,
            timeout,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        receiver.recv().unwrap()
    }

    fn CreateHeap(
        &self,
        heap_type_id: binding::WebGpuHeapTypeId,
        _resource_type: binding::WebGpuResourceType,
        size: u32,
    ) -> Root<WebGpuHeap> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::AllocateMemory {
            gpu_id: self.id,
            desc: w::MemoryDesc {
                size: size as _,
                ty: self.mem_types
                    .iter()
                    .find(|ht| ht.0.id == heap_type_id as _)
                    .map(|ht| &ht.0)
                    .unwrap()
                    .clone(),
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuHeap::new(&self.global(), info)
    }

    fn CreateBuffer(
        &self,
        desc: &binding::WebGpuBufferDesc,
        heap: &WebGpuHeap,
        heap_offset: u32,
    ) -> Root<WebGpuBuffer> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateBuffer {
            gpu_id: self.id,
            desc: w::BufferDesc {
                size: desc.size as _,
                stride: desc.stride as _,
                usage: hal::buffer::Usage::from_bits(desc.usage as _).unwrap(),
                mem_id: heap.get_id(),
                mem_offset: heap_offset as _,
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuBuffer::new(&self.global(), info)
    }

    fn CreateImage(
        &self,
        desc: &binding::WebGpuImageDesc,
        heap: &WebGpuHeap,
        heap_offset: u32,
    ) -> Root<WebGpuImage> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateImage {
            gpu_id: self.id,
            desc: w::ImageDesc {
                kind: hal::image::Kind::D2(
                    desc.width as _,
                    desc.height as _,
                    hal::image::AaMode::Single,
                ),
                levels: 1,
                format: Self::map_format(desc.format),
                usage: hal::image::Usage::from_bits(desc.usage as _).unwrap(),
                mem_id: heap.get_id(),
                mem_offset: heap_offset as _,
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuImage::new(&self.global(), info)
    }

    fn CreateRenderPass(
        &self,
        attachment_descs: Vec<binding::WebGpuAttachmentDesc>,
        subpass_descs: Vec<binding::WebGpuSubpassDesc>,
        dependency_descs: Vec<binding::WebGpuDependency>,
    ) -> Root<WebGpuRenderPass> {
        let attachments = attachment_descs
            .into_iter()
            .map(|at| hal::pass::Attachment {
                format: Self::map_format(at.format),
                layouts: Self::map_image_layout(at.srcLayout) .. Self::map_image_layout(at.dstLayout),
                ops: hal::pass::AttachmentOps::new(Self::map_load_op(at.loadOp), Self::map_store_op(at.storeOp)),
                stencil_ops: hal::pass::AttachmentOps::new(Self::map_load_op(at.stencilLoadOp), Self::map_store_op(at.stencilStoreOp)),
            })
            .collect();

        let subpasses = subpass_descs
            .into_iter()
            .map(|sp| w::SubpassDesc {
                colors: sp
                    .into_iter()
                    .map(|spa| (
                        spa.attachmentId as _,
                        Self::map_image_layout(spa.layout),
                    ))
                    .collect(),
            })
            .collect();

        let dependencies = dependency_descs
            .into_iter()
            .map(|dep| hal::pass::SubpassDependency {
                passes: Self::map_pass_ref(dep.srcPass) ..
                        Self::map_pass_ref(dep.dstPass),
                stages: hal::pso::PipelineStage::from_bits(dep.srcStages as _).unwrap() ..
                        hal::pso::PipelineStage::from_bits(dep.dstStages as _).unwrap(),
                accesses: hal::image::Access::from_bits(dep.srcAccess as _).unwrap() ..
                          hal::image::Access::from_bits(dep.dstAccess as _).unwrap(),
            })
            .collect();

        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateRenderPass {
            gpu_id: self.id,
            desc: w::RenderPassDesc {
                attachments,
                subpasses,
                dependencies,
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuRenderPass::new(&self.global(), info)
    }

    fn CreateFramebuffer(
        &self,
        render_pass: &WebGpuRenderPass,
        size: &binding::WebGpuFramebufferSize,
        attachments: Vec<Root<WebGpuImageView>>,
    ) -> Root<WebGpuFramebuffer> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateFramebuffer {
            gpu_id: self.id,
            desc: w::FramebufferDesc {
                render_pass: render_pass.get_id(),
                attachments: attachments.into_iter().map(|v| v.get_id()).collect(),
                extent: hal::device::Extent {
                    width: size.width as _,
                    height: size.height as _,
                    depth: size.layers as _,
                },
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuFramebuffer::new(&self.global(), info)
    }

    fn CreateDescriptorSetLayout(
        &self,
        bindings: Vec<binding::WebGpuDescriptorSetLayoutBinding>,
    ) -> Root<WebGpuDescriptorSetLayout> {
        let (sender, receiver) = webgpu_channel().unwrap();

        let msg = WebGpuMsg::CreateDescriptorSetLayout {
            gpu_id: self.id,
            bindings: bindings
                .into_iter()
                .map(|b| hal::pso::DescriptorSetLayoutBinding {
                    binding: b.binding as _,
                    ty: Self::map_descriptor_type(b.type_),
                    count: b.count as _,
                    stage_flags: hal::pso::ShaderStageFlags::from_bits(b.stages as _).unwrap(),
                })
                .collect(),
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let layout = receiver.recv().unwrap();
        WebGpuDescriptorSetLayout::new(&self.global(), layout)
    }

    fn CreatePipelineLayout(
        &self,
        set_layouts: Vec<Root<WebGpuDescriptorSetLayout>>,
    ) -> Root<WebGpuPipelineLayout> {
        let (sender, receiver) = webgpu_channel().unwrap();

        let msg = WebGpuMsg::CreatePipelineLayout {
            gpu_id: self.id,
            set_layout_ids: set_layouts
                .into_iter()
                .map(|sl| sl.get_id())
                .collect(),
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let layout = receiver.recv().unwrap();
        WebGpuPipelineLayout::new(&self.global(), layout)
    }

    fn CreateDescriptorPool(
        &self,
        max_sets: u32,
        ranges: Vec<binding::WebGpuDescriptorRange>,
    ) -> Root<WebGpuDescriptorPool> {
        let (sender, receiver) = webgpu_channel().unwrap();

        let msg = WebGpuMsg::CreateDescriptorPool {
            gpu_id: self.id,
            max_sets: max_sets as _,
            ranges: ranges
                .into_iter()
                .map(|r| hal::pso::DescriptorRangeDesc {
                    ty: Self::map_descriptor_type(r.type_),
                    count: r.count as _,
                })
                .collect(),
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let pool = receiver.recv().unwrap();
        WebGpuDescriptorPool::new(&self.global(), self.sender.clone(), pool)
    }

    fn CreateShaderModuleFromGLSL(
        &self,
        ty: binding::WebGpuShaderType,
        code: DOMString,
    ) -> Root<WebGpuShaderModule> {
        use std::io::Read;

        let conv_type = map_enum!(ty; self::binding::WebGpuShaderType =>
            glsl_to_spirv::ShaderType {Vertex, Fragment}
        );
        let mut file = glsl_to_spirv::compile(&code, conv_type).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateShaderModule {
            gpu_id: self.id,
            data,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let module = receiver.recv().unwrap();
        WebGpuShaderModule::new(&self.global(), module)
    }

    fn CreateShaderModuleFromHLSL(
        &self,
        ty: binding::WebGpuShaderType,
        code: DOMString,
    ) -> Root<WebGpuShaderModule> {
        #[cfg(windows)]
        {
            let stage = match ty {
                binding::WebGpuShaderType::Vertex => hal::pso::Stage::Vertex,
                binding::WebGpuShaderType::Fragment => hal::pso::Stage::Fragment,
            };

            let (sender, receiver) = webgpu_channel().unwrap();
            let msg = WebGpuMsg::CreateShaderModuleHLSL {
                gpu_id: self.id,
                stage,
                data: code.as_bytes().to_vec(),
                result: sender,
            };

            self.sender.send(msg).unwrap();
            let module = receiver.recv().unwrap();
            WebGpuShaderModule::new(&self.global(), module)
        }
        #[cfg(not(windows))]
        {
            let _ = (ty, code);
            unimplemented!()
        }
    }

    fn CreateShaderModuleFromMSL(&self, code: DOMString) -> Root<WebGpuShaderModule> {
        #[cfg(target_os = "macos")]
        {
            let (sender, receiver) = webgpu_channel().unwrap();
            let msg = WebGpuMsg::CreateShaderModuleMSL {
                gpu_id: self.id,
                data: code.to_string(),
                result: sender,
            };

            self.sender.send(msg).unwrap();
            let module = receiver.recv().unwrap();
            WebGpuShaderModule::new(&self.global(), module)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = code;
            unimplemented!();
        }
    }

    fn CreateGraphicsPipelines(
        &self,
        descs: Vec<binding::WebGpuGraphicsPipelineDesc>,
    ) -> Vec<Root<WebGpuGraphicsPipeline>> {
        let map_entry_point = |stage: &binding::WebGpuShaderRef| w::EntryPoint {
            module_id: stage.shader_module.get_id(),
            name: stage.entry_point.to_string(),
        };
        let map_input_assembler = |ia: binding::WebGpuInputAssemblyState| hal::pso::InputAssemblerDesc {
            primitive: match ia.topology {
                binding::WebGpuPrimitiveTopology::PointList => hal::Primitive::PointList,
                binding::WebGpuPrimitiveTopology::LineList => hal::Primitive::LineList,
                binding::WebGpuPrimitiveTopology::LineStrip => hal::Primitive::LineStrip,
                binding::WebGpuPrimitiveTopology::TriangleList => hal::Primitive::TriangleList,
                binding::WebGpuPrimitiveTopology::TriangleStrip => hal::Primitive::TriangleStrip,
            },
            primitive_restart: hal::pso::PrimitiveRestart::Disabled, //TODO
        };
        let map_rasterizer = |r: binding::WebGpuRasterizerState| hal::pso::Rasterizer {
            polgyon_mode: match r.polygonMode {
                binding::WebGpuPolygonMode::Fill => hal::state::RasterMethod::Fill,
            },
            cull_mode: hal::state::CullFace::Nothing,
            front_face: match r.frontFace {
                binding::WebGpuFrontFace::Cw => hal::state::FrontFace::Clockwise,
                binding::WebGpuFrontFace::Ccw => hal::state::FrontFace::CounterClockwise,
            },
            depth_clamping: false,
            depth_bias: None,
            conservative: false,
        };
        let map_factor = |factor: binding::WebGpuBlendFactor| match factor {
            binding::WebGpuBlendFactor::Zero => hal::state::Factor::Zero,
            binding::WebGpuBlendFactor::One => hal::state::Factor::One,
            binding::WebGpuBlendFactor::SrcAlpha => hal::state::Factor::ZeroPlus(hal::state::BlendValue::SourceAlpha),
            binding::WebGpuBlendFactor::OneMinusSrcAlpha => hal::state::Factor::OneMinus(hal::state::BlendValue::SourceAlpha),
        };
        let map_channel = |chan: binding::WebGpuBlendChannel| {
            match chan {
                binding::WebGpuBlendChannel {
                    eq: binding::WebGpuBlendEquation::Add,
                    src: binding::WebGpuBlendFactor::One,
                    dst: binding::WebGpuBlendFactor::Zero,
                } => None,
                _ => Some(hal::state::BlendChannel {
                    equation: map_enum!(chan.eq; self::binding::WebGpuBlendEquation =>
                        self::hal::state::Equation {Add, Sub, RevSub, Min, Max}
                    ),
                    source: map_factor(chan.src),
                    destination: map_factor(chan.dst),
                })
            }
        };
        let map_blender = |blend: binding::WebGpuBlendState| hal::pso::BlendDesc {
            alpha_coverage: blend.alphaToCoverage,
            logic_op: None, //TODO
            targets: blend.targets.into_iter().map(|target| hal::pso::ColorInfo {
                mask: hal::state::ColorMask::from_bits(target.mask as _).unwrap(),
                color: map_channel(target.color),
                alpha: map_channel(target.alpha),
            }).collect(),
        };

        let descriptors = descs
            .into_iter()
            .flat_map(|desc| Some(w::GraphicsPipelineDesc {
                shaders: w::GraphicsShaderSet {
                    vs: match desc.shaders.get("vs") {
                        Some(ref entry) => map_entry_point(entry),
                        None => return None,
                    },
                    fs: desc.shaders.get("fs").map(&map_entry_point),
                },
                layout_id: desc.layout.get_id(),
                renderpass_id: desc.renderPass.get_id(),
                subpass: desc.subpass,
                inner: hal::pso::GraphicsPipelineDesc {
                    rasterizer: map_rasterizer(desc.rasterizerState),
                    vertex_buffers: Vec::new(), //TODO
                    attributes: Vec::new(), //TODO
                    input_assembler: map_input_assembler(desc.inputAssemblyState),
                    blender: map_blender(desc.blendState),
                    depth_stencil: None, //TODO
                },
            }))
            .collect::<Vec<_>>();

        let count = descriptors.len();
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateGraphicsPipelines {
            gpu_id: self.id,
            descriptors,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        (0..count).map(|_| {
            let info = receiver.recv().unwrap();
            WebGpuGraphicsPipeline::new(&self.global(), info)
        }).collect()
    }

    fn CreateSampler(
        &self,
        desc: &binding::WebGpuSamplerDesc,
    ) -> Root<WebGpuSampler> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateSampler {
            gpu_id: self.id,
            desc: hal::image::SamplerInfo::new(
                map_enum!(desc.filter;
                    self::binding::WebGpuFilterMode => self::hal::image::FilterMethod {
                        Scale, Mipmap, Bilinear, Trilinear
                    }
                ),
                map_enum!(desc.wrap;
                    self::binding::WebGpuWrapMode => self::hal::image::WrapMode {
                        Tile, Mirror, Clamp, Border
                    }
                ),
            ),
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuSampler::new(&self.global(), info)
    }

    fn CreateImageView(
        &self,
        image: &WebGpuImage,
        format: binding::WebGpuFormat,
    ) -> Root<WebGpuImageView> {
        let range = hal::image::SubresourceRange {
            aspects: hal::image::ASPECT_COLOR, //TODO
            levels: 0 .. 1,
            layers: 0 .. 1,
        };

        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateImageView {
            gpu_id: self.id,
            image_id: image.get_id(),
            format: Self::map_format(format),
            range,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuImageView::new(&self.global(), info)
    }

    #[allow(unsafe_code)]
    unsafe fn UploadBufferData(&self, cx: *mut JSContext, buffer: &WebGpuBuffer, data: *mut JSObject) {
        typedarray!(in(cx) let array_buffer: ArrayBuffer = data);
        typedarray!(in(cx) let array_buffer_view: ArrayBufferView = data);
        let data_vec = match array_buffer {
            Ok(mut data) => data.as_slice().to_vec(),
            Err(_) => match array_buffer_view {
                Ok(mut v) => v.as_slice().to_vec(),
                Err(_) => panic!("Unsupported data for UploadBufferData")
            }
        };

        let msg = WebGpuMsg::UploadBufferData {
            gpu_id: self.id,
            buffer_id: buffer.get_id(),
            data: data_vec,
        };
        self.sender.send(msg).unwrap();
    }

    fn UpdateDescriptorSets(&self, writes: Vec<binding::WebGpuDescriptorSetWrite>) {
        let msg = WebGpuMsg::UpdateDescriptorSets {
            gpu_id: self.id,
            writes: writes
                .into_iter()
                .map(|w| w::DescriptorSetWrite {
                    set: w.set.get_id(),
                    binding: w.binding as _,
                    array_offset: w.arrayOffset as _,
                    ty: Self::map_descriptor_type(w.type_),
                    descriptors: w.descriptors
                        .into_iter()
                        .map(|desc| (
                            desc.target.get_id(),
                            Self::map_image_layout(desc.layout),
                        ))
                        .collect(),
                })
                .collect(),
        };
        self.sender.send(msg).unwrap();
    }
}
