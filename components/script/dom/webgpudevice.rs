/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{self as w, gpu, webgpu_channel,
    WebGpuChan, WebGpuMsg};
use dom::bindings::codegen::Bindings::WebGpuDeviceBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::globalscope::GlobalScope;
use dom::webgpubuffer::WebGpuBuffer;
use dom::webgpudepthstencilview::WebGpuDepthStencilView;
use dom::webgpufence::WebGpuFence;
use dom::webgpuframebuffer::WebGpuFramebuffer;
use dom::webgpugraphicspipeline::WebGpuGraphicsPipeline;
use dom::webgpuheap::WebGpuHeap;
use dom::webgpuimage::WebGpuImage;
use dom::webgpupipelinelayout::WebGpuPipelineLayout;
use dom::webgpurenderpass::WebGpuRenderpass;
use dom::webgpurendertargetview::WebGpuRenderTargetView;
use dom::webgpushadermodule::WebGpuShaderModule;
use dom_struct::dom_struct;
use heapsize::HeapSizeOf;
use js::jsapi::{JSContext, JSObject};
use glsl_to_spirv;


pub struct LimitsWrapper(pub gpu::Limits);
impl HeapSizeOf for LimitsWrapper {
    fn heap_size_of_children(&self) -> usize { 0 }
}
pub struct HeapTypeWrapper(pub gpu::HeapType);
impl HeapSizeOf for HeapTypeWrapper {
    fn heap_size_of_children(&self) -> usize { 0 }
}


#[dom_struct]
pub struct WebGpuDevice {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuChan,
    id: w::GpuId,
    limits: LimitsWrapper,
    heap_types: Vec<HeapTypeWrapper>,
}

impl WebGpuDevice {
    pub fn new(
        global: &GlobalScope,
        sender: WebGpuChan,
        id: w::GpuId,
        limits: gpu::Limits,
        heap_types: &[gpu::HeapType],
    ) -> Root<Self>
    {
        let obj = box WebGpuDevice {
            reflector_: Reflector::new(),
            sender,
            id,
            limits: LimitsWrapper(limits),
            heap_types: heap_types
                .iter()
                .map(|ht| HeapTypeWrapper(ht.clone()))
                .collect(),
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }

    pub fn map_image_layout(layout: binding::WebGpuImageLayout) -> gpu::image::ImageLayout {
        map_enum!(layout; self::binding::WebGpuImageLayout => self::gpu::image::ImageLayout {
            General, ColorAttachmentOptimal, DepthStencilAttachmentOptimal,
            DepthStencilReadOnlyOptimal, ShaderReadOnlyOptimal,
            TransferSrcOptimal, TransferDstOptimal,
            Undefined, Preinitialized, Present
        })
    }

    pub fn map_format(format: binding::WebGpuFormat) -> gpu::format::Format {
        use self::binding::WebGpuFormat::*;
        use self::gpu::format::{Format, SurfaceType, ChannelType};
        match format {
            R8G8B8A8_UNORM => Format(SurfaceType::R8_G8_B8_A8, ChannelType::Unorm),
            R8G8B8A8_SRGB => Format(SurfaceType::R8_G8_B8_A8, ChannelType::Srgb),
            B8G8R8A8_UNORM => Format(SurfaceType::B8_G8_R8_A8, ChannelType::Unorm),
            B8G8R8A8_SRGB => Format(SurfaceType::B8_G8_R8_A8, ChannelType::Srgb),
        }
    }

    fn map_load_op(op: binding::WebGpuAttachmentLoadOp) -> gpu::pass::AttachmentLoadOp {
        use self::binding::WebGpuAttachmentLoadOp::*;
        use self::gpu::pass::AttachmentLoadOp as Alo;
        match op {
            Load => Alo::Load,
            Clear => Alo::Clear,
            DontCare => Alo::DontCare,
        }
    }

    fn map_store_op(op: binding::WebGpuAttachmentStoreOp) -> gpu::pass::AttachmentStoreOp {
        use self::binding::WebGpuAttachmentStoreOp::*;
        use self::gpu::pass::AttachmentStoreOp as Aso;
        match op {
            Store => Aso::Store,
            DontCare => Aso::DontCare,
        }
    }

    fn map_pass_ref(pass_ref: Option<u32>) -> gpu::pass::SubpassRef {
        match pass_ref {
            Some(id) => gpu::pass::SubpassRef::Pass(id as _),
            None => gpu::pass::SubpassRef::External,
        }
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
            binding::WebGpuFenceWait::Any => gpu::device::WaitFor::Any,
            binding::WebGpuFenceWait::All => gpu::device::WaitFor::All,
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
        resource_type: binding::WebGpuResourceType,
        size: u32,
    ) -> Root<WebGpuHeap> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateHeap {
            gpu_id: self.id,
            desc: w::HeapDesc {
                size: size as _,
                ty: self.heap_types
                    .iter()
                    .find(|ht| ht.0.id == heap_type_id as _)
                    .map(|ht| &ht.0)
                    .unwrap()
                    .clone(),
                resources: map_enum!(resource_type; self::binding::WebGpuResourceType =>
                    self::gpu::device::ResourceHeapType {Any, Buffers, Images, Targets}
                ),
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
                usage: gpu::buffer::Usage::from_bits(desc.usage as _).unwrap(),
                heap_id: heap.get_id(),
                heap_offset: heap_offset as _,
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
                kind: gpu::image::Kind::D2(
                    desc.width as _,
                    desc.height as _,
                    gpu::image::AaMode::Single,
                ),
                levels: 1,
                format: Self::map_format(desc.format),
                usage: gpu::image::Usage::from_bits(desc.usage as _).unwrap(),
                heap_id: heap.get_id(),
                heap_offset: heap_offset as _,
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuImage::new(&self.global(), info)
    }

    fn CreateRenderpass(
        &self,
        attachment_descs: Vec<binding::WebGpuAttachmentDesc>,
        subpass_descs: Vec<binding::WebGpuSubpassDesc>,
        dependency_descs: Vec<binding::WebGpuDependency>,
    ) -> Root<WebGpuRenderpass> {
        let attachments = attachment_descs
            .into_iter()
            .map(|at| gpu::pass::Attachment {
                format: Self::map_format(at.format),
                layouts: Self::map_image_layout(at.srcLayout) .. Self::map_image_layout(at.dstLayout),
                ops: gpu::pass::AttachmentOps::new(Self::map_load_op(at.loadOp), Self::map_store_op(at.storeOp)),
                stencil_ops: gpu::pass::AttachmentOps::new(Self::map_load_op(at.stencilLoadOp), Self::map_store_op(at.stencilStoreOp)),
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
            .map(|dep| gpu::pass::SubpassDependency {
                passes: Self::map_pass_ref(dep.srcPass) ..
                        Self::map_pass_ref(dep.dstPass),
                stages: gpu::pso::PipelineStage::from_bits(dep.srcStages as _).unwrap() ..
                        gpu::pso::PipelineStage::from_bits(dep.dstStages as _).unwrap(),
                accesses: gpu::image::Access::from_bits(dep.srcAccess as _).unwrap() ..
                          gpu::image::Access::from_bits(dep.dstAccess as _).unwrap(),
            })
            .collect();

        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateRenderpass {
            gpu_id: self.id,
            desc: w::RenderpassDesc {
                attachments,
                subpasses,
                dependencies,
            },
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuRenderpass::new(&self.global(), info)
    }

    fn CreateFramebuffer(
        &self,
        renderpass: &WebGpuRenderpass,
        size: &binding::WebGpuFramebufferSize,
        colors: Vec<Root<WebGpuRenderTargetView>>,
        depth_stencil: Option<&WebGpuDepthStencilView>,
    ) -> Root<WebGpuFramebuffer> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreateFramebuffer {
            gpu_id: self.id,
            desc: w::FramebufferDesc {
                renderpass: renderpass.get_id(),
                colors: colors.into_iter().map(|v| v.get_id()).collect(),
                depth_stencil: depth_stencil.map(|v| v.get_id()),
                extent: gpu::device::Extent {
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

    fn CreatePipelineLayout(
        &self,
        _sets: Vec<binding::WebGpuDescriptorSetLayout>,
    ) -> Root<WebGpuPipelineLayout> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::CreatePipelineLayout {
            gpu_id: self.id,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let layout = receiver.recv().unwrap();
        WebGpuPipelineLayout::new(&self.global(), layout)
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

    fn CreateGraphicsPipelines(
        &self,
        descs: Vec<binding::WebGpuGraphicsPipelineDesc>,
    ) -> Vec<Root<WebGpuGraphicsPipeline>> {
        let map_entry_point = |stage: &binding::WebGpuShaderStage| w::EntryPoint {
            module_id: stage.shader_module.get_id(),
            name: stage.entry_point.to_string(),
        };
        let map_input_assembler = |ia: binding::WebGpuInputAssemblyState| gpu::pso::InputAssemblerDesc {
            primitive: match ia.topology {
                binding::WebGpuPrimitiveTopology::PointList => gpu::Primitive::PointList,
                binding::WebGpuPrimitiveTopology::LineList => gpu::Primitive::LineList,
                binding::WebGpuPrimitiveTopology::LineStrip => gpu::Primitive::LineStrip,
                binding::WebGpuPrimitiveTopology::TriangleList => gpu::Primitive::TriangleList,
                binding::WebGpuPrimitiveTopology::TriangleStrip => gpu::Primitive::TriangleStrip,
            },
            primitive_restart: gpu::pso::PrimitiveRestart::Disabled, //TODO
        };
        let map_rasterizer = |r: binding::WebGpuRasterizerState| gpu::pso::Rasterizer {
            polgyon_mode: match r.polygonMode {
                binding::WebGpuPolygonMode::Fill => gpu::state::RasterMethod::Fill,
            },
            cull_mode: gpu::state::CullFace::Nothing,
            front_face: match r.frontFace {
                binding::WebGpuFrontFace::Cw => gpu::state::FrontFace::Clockwise,
                binding::WebGpuFrontFace::Ccw => gpu::state::FrontFace::CounterClockwise,
            },
            depth_clamping: false,
            depth_bias: None,
            conservative: false,
        };
        let map_factor = |factor: binding::WebGpuBlendFactor| match factor {
            binding::WebGpuBlendFactor::Zero => gpu::state::Factor::Zero,
            binding::WebGpuBlendFactor::One => gpu::state::Factor::One,
            binding::WebGpuBlendFactor::SrcAlpha => gpu::state::Factor::ZeroPlus(gpu::state::BlendValue::SourceAlpha),
            binding::WebGpuBlendFactor::OneMinusSrcAlpha => gpu::state::Factor::OneMinus(gpu::state::BlendValue::SourceAlpha),
        };
        let map_channel = |chan: binding::WebGpuBlendChannel| {
            match chan {
                binding::WebGpuBlendChannel {
                    eq: binding::WebGpuBlendEquation::Add,
                    src: binding::WebGpuBlendFactor::One,
                    dst: binding::WebGpuBlendFactor::Zero,
                } => None,
                _ => Some(gpu::state::BlendChannel {
                    equation: map_enum!(chan.eq; self::binding::WebGpuBlendEquation =>
                        self::gpu::state::Equation {Add, Sub, RevSub, Min, Max}
                    ),
                    source: map_factor(chan.src),
                    destination: map_factor(chan.dst),
                })
            }
        };
        let map_blender = |blend: binding::WebGpuBlendState| gpu::pso::BlendDesc {
            alpha_coverage: blend.alphaToCoverage,
            logic_op: None, //TODO
            targets: blend.targets.into_iter().map(|target| gpu::pso::ColorInfo {
                mask: gpu::state::ColorMask::from_bits(target.mask as _).unwrap(),
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
                renderpass_id: desc.renderpass.get_id(),
                subpass: desc.subpass,
                inner: gpu::pso::GraphicsPipelineDesc {
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

    fn ViewImageAsRenderTarget(
        &self,
        image: &WebGpuImage,
        format: binding::WebGpuFormat,
    ) -> Root<WebGpuRenderTargetView> {
        let (sender, receiver) = webgpu_channel().unwrap();
        let msg = WebGpuMsg::ViewImageAsRenderTarget {
            gpu_id: self.id,
            image_id: image.get_id(),
            format: Self::map_format(format),
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGpuRenderTargetView::new(&self.global(), info)
    }

    #[allow(unsafe_code)]
    unsafe fn UploadBufferData(&self, cx: *mut JSContext, buffer: &WebGpuBuffer, data: *mut JSObject) -> () {
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

        ()
    }
}
