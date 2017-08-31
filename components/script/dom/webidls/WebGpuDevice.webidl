/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
 
typedef unsigned long WebGpuSemaphore;
typedef unsigned long WebGpuRenderTargetView;
typedef unsigned long WebGpuDepthStencilView;

enum WebGpuFormat {
	"R8G8B8A8_UNORM",
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
	required long srcPass;
	required long dstPass;
	required WebGpuAccess srcAccess;
	required WebGpuAccess dstAccess;
	//required WebGpuPipelineStage srcStage;
	//required WebGpuPipelineStage dstStage;
};


interface WebGpuDevice {
	readonly attribute WebGpuCommandQueue generalQueue; //TODO: FrozenArray<>

	WebGpuFence createFence(boolean set);
	void resetFences(sequence<WebGpuFence> fences);
	void waitForFences(
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
		sequence<WebGpuRenderTargetView> colors,
		WebGpuDepthStencilView? depth_stencil
	);

	WebGpuRenderTargetView viewImageAsRenderTarget(WebGpuImage image);
};
