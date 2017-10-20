/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

typedef unsigned long WebGpuBufferUsage;
typedef unsigned long WebGpuBufferAccess;
typedef unsigned long WebGpuImageUsage;
typedef unsigned long WebGpuImageAccess;
typedef unsigned long WebGpuPipelineStage;
typedef unsigned long WebGpuHeapProperty;
typedef unsigned short WebGpuHeapTypeId;
typedef unsigned long WebGpuDescriptorBinding;
typedef unsigned long WebGpuShaderStage;
typedef WebGpuDescriptor WebGpuDescriptor_;

enum WebGpuFormat {
	"R8G8B8A8_UNORM",
	"R8G8B8A8_SRGB",
	"B8G8R8A8_UNORM",
	"B8G8R8A8_SRGB",
};

enum WebGpuImageLayout {
	"General",
	"ColorAttachmentOptimal",
	"DepthStencilAttachmentOptimal",
	"DepthStencilReadOnlyOptimal",
	"ShaderReadOnlyOptimal",
	"TransferSrcOptimal",
	"TransferDstOptimal",
	"Undefined",
	"Preinitialized",
	"Present",
};

enum WebGpuFenceWait {
	"Any",
	"All",
};

enum WebGpuAttachmentLoadOp {
	"Load",
	"Clear",
	"DontCare",
};

enum WebGpuAttachmentStoreOp {
	"Store",
	"DontCare",
};

enum WebGpuShaderType {
	"Vertex",
	"Fragment",
};

enum WebGpuPrimitiveTopology {
	"PointList",
	"LineList",
	"LineStrip",
	"TriangleList",
	"TriangleStrip",
};

enum WebGpuPolygonMode {
	"Fill",
};

enum WebGpuFrontFace {
	"Cw",
	"Ccw",
};

enum WebGpuBlendEquation {
	"Add",
	"Sub",
	"RevSub",
	"Min",
	"Max",
};

enum WebGpuBlendFactor {
	"Zero",
	"One",
	"SrcAlpha",
	"OneMinusSrcAlpha",
	//TODO
};

enum WebGpuResourceType {
	"Any",
	"Buffers",
	"Images",
	"Targets",
};

enum WebGpuDescriptorType {
	"Sampler",
	"SampledImage",
	"StorageImage",
	"UniformTexelBuffer",
	"StorageTexelBuffer",
	"UniformBuffer",
	"StorageBuffer",
	"InputAttachment",
};

enum WebGpuFilterMode {
	"Scale",
	"Mipmap",
	"Bilinear",
	"Trilinear",
};

enum WebGpuWrapMode {
	"Tile",
	"Mirror",
	"Clamp",
	"Border",
};


dictionary WebGpuDeviceLimits {
	required unsigned long minBufferCopyOffsetAlignment;
	required unsigned long minBufferCopyPitchAlignment;
};

dictionary WebGpuAttachmentDesc {
	required WebGpuFormat format;
	required WebGpuImageLayout srcLayout;
	required WebGpuImageLayout dstLayout;
	required WebGpuAttachmentLoadOp loadOp;
	required WebGpuAttachmentStoreOp storeOp;
	WebGpuAttachmentLoadOp stencilLoadOp = "DontCare";
	WebGpuAttachmentStoreOp stencilStoreOp = "DontCare";
};

dictionary WebGpuSubpassAttachment {
	required unsigned long attachmentId;
	required WebGpuImageLayout layout;
};

typedef sequence<WebGpuSubpassAttachment> WebGpuSubpassDesc;

dictionary WebGpuDependency {
	required unsigned long? srcPass;
	required unsigned long? dstPass;
	required WebGpuImageAccess srcAccess;
	required WebGpuImageAccess dstAccess;
	required WebGpuPipelineStage srcStages;
	required WebGpuPipelineStage dstStages;
};

dictionary WebGpuFramebufferSize {
	required unsigned long width;
	required unsigned long height;
	required unsigned long layers;
};

dictionary WebGpuShaderRef {
	required WebGpuShaderModule shader_module; //Note: "module" is a keyword
	required DOMString entry_point;
};

dictionary WebGpuInputAssemblyState {
	required WebGpuPrimitiveTopology topology;
};

dictionary WebGpuRasterizerState {
	WebGpuPolygonMode polygonMode = "Fill";
	WebGpuFrontFace frontFace = "Ccw";
};

dictionary WebGpuBlendChannel {
	WebGpuBlendEquation eq = "Add";
	WebGpuBlendFactor src = "One";
	WebGpuBlendFactor dst = "Zero";
};

dictionary WebGpuColorTarget {
	unsigned long mask = 0xF;
	required WebGpuBlendChannel color;
	required WebGpuBlendChannel alpha;
};

dictionary WebGpuBlendState {
	boolean alphaToCoverage = false;
	//TODO: logicOp
	required sequence<WebGpuColorTarget> targets;
};

dictionary WebGpuDescriptorSetLayoutBinding {
	required WebGpuDescriptorBinding binding;
	required WebGpuDescriptorType type;
	required unsigned long count;
	WebGpuShaderStage stages = 0xFF;
};

dictionary WebGpuGraphicsPipelineDesc {
	required record<DOMString, WebGpuShaderRef> shaders; // `WebGpuShaderType`
	required WebGpuInputAssemblyState inputAssemblyState;
	required WebGpuRasterizerState rasterizerState;
	required WebGpuBlendState blendState;
	required WebGpuPipelineLayout layout;
	required WebGpuRenderPass renderPass;
	unsigned long subpass = 0;
};

dictionary WebGpuDescriptorRange {
	required WebGpuDescriptorType type;
	required unsigned long count;
};

dictionary WebGpuBufferDesc {
	required unsigned long size;
	required unsigned long stride;
	required WebGpuBufferUsage usage;
};

dictionary WebGpuImageDesc {
	required unsigned long width;
	required unsigned long height;
	required WebGpuFormat format;
	required WebGpuImageUsage usage;
};

dictionary WebGpuSamplerDesc {
	WebGpuFilterMode filter = "Trilinear";
	required WebGpuWrapMode wrap;
};

dictionary WebGpuDescriptorWrite {
	required WebGpuDescriptor target;
	WebGpuImageLayout layout = "Undefined";
};

dictionary WebGpuDescriptorSetWrite {
	required WebGpuDescriptorSet set;
	required WebGpuDescriptorBinding binding;
	required unsigned long arrayOffset;
	required WebGpuDescriptorType type;
	required sequence<WebGpuDescriptorWrite> descriptors;
};


interface WebGpuDevice {
	// buffer usage flags
	const WebGpuBufferUsage	BUFFER_USAGE_TRANSFER_SRC 	= 0x1;
	const WebGpuBufferUsage	BUFFER_USAGE_TRANSFER_DST 	= 0x2;
	const WebGpuBufferUsage	BUFFER_USAGE_CONSTANT 		= 0x4;
	const WebGpuBufferUsage	BUFFER_USAGE_INDEX 			= 0x8;
	const WebGpuBufferUsage	BUFFER_USAGE_VERTEX 		= 0x10;
	const WebGpuBufferUsage	BUFFER_USAGE_INDIRECT 		= 0x20;

	const WebGpuImageUsage IMAGE_USAGE_TRANSFER_SRC				= 0x01;
	const WebGpuImageUsage IMAGE_USAGE_TRANSFER_DST				= 0x02;
	const WebGpuImageUsage IMAGE_USAGE_COLOR_ATTACHMENT			= 0x04;
	const WebGpuImageUsage IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT	= 0x08;
	const WebGpuImageUsage IMAGE_USAGE_STORAGE					= 0x10;
	const WebGpuImageUsage IMAGE_USAGE_SAMPLED					= 0x20;

	// buffer access flags
	const WebGpuBufferAccess BUFFER_ACCESS_TRANSFER_READ			= 0x01;
	const WebGpuBufferAccess BUFFER_ACCESS_TRANSFER_WRITE			= 0x02;
	const WebGpuBufferAccess BUFFER_ACCESS_INDEX_BUFFER_READ		= 0x10;
	const WebGpuBufferAccess BUFFER_ACCESS_VERTEX_BUFFER_READ		= 0x20;
	const WebGpuBufferAccess BUFFER_ACCESS_CONSTANT_BUFFER_READ		= 0x40;
	const WebGpuBufferAccess BUFFER_ACCESS_INDIRECT_COMMAND_READ	= 0x80;
	// image access flags
	const WebGpuImageAccess IMAGE_ACCESS_COLOR_ATTACHMENT_READ  = 0x1;
	const WebGpuImageAccess IMAGE_ACCESS_COLOR_ATTACHMENT_WRITE = 0x2;
	const WebGpuImageAccess IMAGE_ACCESS_TRANSFER_READ          = 0x4;
	const WebGpuImageAccess IMAGE_ACCESS_TRANSFER_WRITE         = 0x8;
	const WebGpuImageAccess IMAGE_ACCESS_SHADER_READ            = 0x10;
	const WebGpuImageAccess IMAGE_ACCESS_RENDER_TARGET_CLEAR    = 0x20;
	const WebGpuImageAccess IMAGE_ACCESS_RESOLVE_SRC            = 0x100;
	const WebGpuImageAccess IMAGE_ACCESS_RESOLVE_DST            = 0x200;

	const WebGpuPipelineStage PIPELINE_STAGE_TOP_OF_PIPE				= 0x1;
	const WebGpuPipelineStage PIPELINE_STAGE_DRAW_INDIRECT				= 0x2;
	const WebGpuPipelineStage PIPELINE_STAGE_VERTEX_INPUT				= 0x4;
	const WebGpuPipelineStage PIPELINE_STAGE_VERTEX_SHADER				= 0x8;
	const WebGpuPipelineStage PIPELINE_STAGE_HULL_SHADER				= 0x10;
	const WebGpuPipelineStage PIPELINE_STAGE_DOMAIN_SHADER				= 0x20;
	const WebGpuPipelineStage PIPELINE_STAGE_GEOMETRY_SHADER			= 0x40;
	const WebGpuPipelineStage PIPELINE_STAGE_FRAGMENT_SHADER			= 0x80;
	const WebGpuPipelineStage PIPELINE_STAGE_EARLY_FRAGMENT_TESTS		= 0x100;
	const WebGpuPipelineStage PIPELINE_STAGE_LATE_FRAGMENT_TESTS		= 0x200;
	const WebGpuPipelineStage PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT	= 0x400;
	const WebGpuPipelineStage PIPELINE_STAGE_COMPUTE_SHADER				= 0x800;
	const WebGpuPipelineStage PIPELINE_STAGE_TRANSFER					= 0x1000;
	const WebGpuPipelineStage PIPELINE_STAGE_BOTTOM_OF_PIPE				= 0x2000;
	const WebGpuPipelineStage PIPELINE_STAGE_HOST						= 0x4000;

	const WebGpuShaderStage SHADER_STAGE_VERTEX   = 0x01;
	const WebGpuShaderStage SHADER_STAGE_HULL     = 0x02;
	const WebGpuShaderStage SHADER_STAGE_DOMAIN   = 0x04;
	const WebGpuShaderStage SHADER_STAGE_GEOMETRY = 0x08;
	const WebGpuShaderStage SHADER_STAGE_FRAGMENT = 0x10;
	const WebGpuShaderStage SHADER_STAGE_COMPUTE  = 0x20;
	const WebGpuShaderStage SHADER_STAGE_GRAPHICS = 0x1F;
	const WebGpuShaderStage SHADER_STAGE_ALL      = 0x3F;

	const WebGpuHeapProperty HEAP_PROPERTY_DEVICE_LOCAL					= 0x01;
	const WebGpuHeapProperty HEAP_PROPERTY_COHERENT						= 0x02;
	const WebGpuHeapProperty HEAP_PROPERTY_CPU_VISIBLE					= 0x04;
	const WebGpuHeapProperty HEAP_PROPERTY_CPU_CACHED					= 0x08;
	const WebGpuHeapProperty HEAP_PROPERTY_WRITE_COMBINED				= 0x10;


	WebGpuDeviceLimits getLimits();

	WebGpuFence createFence(boolean set);
	void resetFences(sequence<WebGpuFence> fences);
	boolean waitForFences(
		sequence<WebGpuFence> fences,
		WebGpuFenceWait mode,
		unsigned long timeout
	);

	WebGpuHeap createHeap(
		WebGpuHeapTypeId heapTypeId,
		WebGpuResourceType resourceType,
		unsigned long size
	);

	WebGpuBuffer createBuffer(
		WebGpuBufferDesc desc,
		WebGpuHeap heap,
		unsigned long heap_offset
	);

	WebGpuImage createImage(
		WebGpuImageDesc desc,
		WebGpuHeap heap,
		unsigned long heap_offset
	);

	WebGpuImageView createImageView(
		WebGpuImage image,
		WebGpuFormat format
	);

	WebGpuRenderPass createRenderPass(
		sequence<WebGpuAttachmentDesc> attachments,
		sequence<WebGpuSubpassDesc> subpasses,
		sequence<WebGpuDependency> dependencies
	);

	WebGpuFramebuffer createFramebuffer(
		WebGpuRenderPass renderPass,
		WebGpuFramebufferSize size,
		sequence<WebGpuImageView> attachments
	);

	WebGpuDescriptorSetLayout createDescriptorSetLayout(
		sequence<WebGpuDescriptorSetLayoutBinding> bindings
	);

	WebGpuPipelineLayout createPipelineLayout(
		sequence<WebGpuDescriptorSetLayout> sets
	);

	WebGpuDescriptorPool createDescriptorPool(
		unsigned long maxSets,
		sequence<WebGpuDescriptorRange> ranges
	);

	WebGpuShaderModule createShaderModuleFromGLSL(
		WebGpuShaderType stage,
		DOMString code
	);

	WebGpuShaderModule createShaderModuleFromHLSL(
		WebGpuShaderType stage,
		DOMString code
	);

	WebGpuShaderModule createShaderModuleFromMSL(DOMString code);

	sequence<WebGpuGraphicsPipeline> createGraphicsPipelines(
		sequence<WebGpuGraphicsPipelineDesc> descriptors
	);

	WebGpuSampler createSampler(WebGpuSamplerDesc desc);

	void uploadBufferData(WebGpuBuffer buffer, object data);

	void updateDescriptorSets(sequence<WebGpuDescriptorSetWrite> writes);
};
