/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
 
typedef unsigned long WebGpuBufferAccess;
typedef unsigned long WebGpuImageAccess;
typedef unsigned long WebGpuPipelineStage;

enum WebGpuFormat {
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


dictionary WebGpuDescriptorSetLayout {
	//TODO
};

dictionary WebGpuShaderStage {
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

dictionary WebGpuGraphicsPipelineDesc {
	required record<DOMString, WebGpuShaderStage> shaders; // `WebGpuShaderType`
	required WebGpuInputAssemblyState inputAssemblyState;
	required WebGpuRasterizerState rasterizerState;
	required WebGpuBlendState blendState;
	required WebGpuPipelineLayout layout;
	required WebGpuRenderpass renderpass;
	unsigned long subpass = 0;
};


interface WebGpuDevice {
	/* Vulkan original:
	const WebGpuAccess ACCESS_INDIRECT_COMMAND_READ			= 0x0001;
	const WebGpuAccess ACCESS_INDEX_BUFFER_READ				= 0x0002;
	const WebGpuAccess ACCESS_VERTEX_ATTRIBUTE_READ			= 0x0004;
	const WebGpuAccess ACCESS_UNIFORM_READ					= 0x0008;
	const WebGpuAccess ACCESS_INPUT_ATTACHMENT_READ			= 0x0010;
	const WebGpuAccess ACCESS_SHADER_READ					= 0x0020;
	const WebGpuAccess ACCESS_SHADER_WRITE					= 0x0040;
	const WebGpuAccess ACCESS_COLOR_ATTACHMENT_READ			= 0x0080;
	const WebGpuAccess ACCESS_COLOR_ATTACHMENT_WRITE		= 0x0100;
	const WebGpuAccess ACCESS_DEPTH_STENCILATTACHMENT_READ	= 0x0200;
	const WebGpuAccess ACCESS_DEPTH_STENCILATTACHMENT_WRITE	= 0x0400;
	const WebGpuAccess ACCESS_TRANSFER_READ					= 0x0800;
	const WebGpuAccess ACCESS_TRANSFER_WRITE				= 0x1000;
	const WebGpuAccess ACCESS_HOST_READ						= 0x2000;
	const WebGpuAccess ACCESS_HOST_WRITE					= 0x4000;
	const WebGpuAccess ACCESS_MEMORY_READ					= 0x8000;
	const WebGpuAccess ACCESS_MEMORY_WRITE					= 0x10000;
	*/
	// buffer access flags
	const WebGpuBufferAccess	ACCESS_INDEX_BUFFER_READ      = 0x1;
	const WebGpuBufferAccess	ACCESS_VERTEX_BUFFER_READ     = 0x2;
	const WebGpuBufferAccess	ACCESS_CONSTANT_BUFFER_READ   = 0x4;
	const WebGpuBufferAccess	ACCESS_INDIRECT_COMMAND_READ  = 0x8;
	// image access flags
	const WebGpuImageAccess		ACCESS_COLOR_ATTACHMENT_READ  = 0x1;
	const WebGpuImageAccess 	ACCESS_COLOR_ATTACHMENT_WRITE = 0x2;
	const WebGpuImageAccess 	ACCESS_TRANSFER_READ          = 0x4;
	const WebGpuImageAccess 	ACCESS_TRANSFER_WRITE         = 0x8;
	const WebGpuImageAccess 	ACCESS_SHADER_READ            = 0x10;
	const WebGpuImageAccess 	ACCESS_RENDER_TARGET_CLEAR    = 0x20;
	const WebGpuImageAccess 	ACCESS_RESOLVE_SRC            = 0x100;
	const WebGpuImageAccess 	ACCESS_RESOLVE_DST            = 0x200;

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

	readonly attribute WebGpuCommandQueue generalQueue; //TODO: FrozenArray<>

	WebGpuFence createFence(boolean set);
	void resetFences(sequence<WebGpuFence> fences);
	boolean waitForFences(
		sequence<WebGpuFence> fences,
		WebGpuFenceWait mode,
		unsigned long timeout
	);

	WebGpuRenderpass createRenderpass(
		sequence<WebGpuAttachmentDesc> attachments,
		sequence<WebGpuSubpassDesc> subpasses,
		sequence<WebGpuDependency> dependencies
	);

	WebGpuFramebuffer createFramebuffer(
		WebGpuRenderpass renderpass,
		WebGpuFramebufferSize size,
		sequence<WebGpuRenderTargetView> colors,
		WebGpuDepthStencilView? depth_stencil
	);

	WebGpuPipelineLayout createPipelineLayout(
		sequence<WebGpuDescriptorSetLayout> sets
	);

	WebGpuShaderModule createShaderModuleFromGLSL(
		WebGpuShaderType stage,
		DOMString code
	);

	sequence<WebGpuGraphicsPipeline> createGraphicsPipelines(
		sequence<WebGpuGraphicsPipelineDesc> descriptors
	);

	WebGpuRenderTargetView viewImageAsRenderTarget(
		WebGpuImage image,
		WebGpuFormat format
	);
};
